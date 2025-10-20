use crate::app_constants::SESSION_STORAGE_PATH;
use crate::session_storage::internal::data::{SessionsData, UserSessionData, UserSessionsData};
use crate::session_storage::internal::io_trait::{ProductionSessionStorageIo, SessionStorageIo};
use crate::session_storage::internal::session::Session;
use crate::session_storage::SessionStorageError;
use async_trait::async_trait;
use dumbnotes::config::app_config::AppConfig;
use dumbnotes::username_string::{UsernameStr, UsernameString};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::sync::RwLock;
use uuid::Uuid;

#[cfg(test)] mod tests;
mod data;
pub(super) mod session;
mod io_trait;

#[async_trait]
pub trait SessionStorage: Send + Sync {
    async fn create_session(
        &self,
        username: &UsernameStr,
        created_at: OffsetDateTime,
        expires_at: OffsetDateTime,
    ) -> Result<Session, SessionStorageError>;

    async fn refresh_session(
        &self,
        refresh_token: &[u8],
        expires_at: OffsetDateTime,
    ) -> Result<Session, SessionStorageError>;

    async fn delete_session(
        &self,
        session_id: Uuid,
    ) -> Result<bool, SessionStorageError>;

    async fn get_session_by_id(
        &self,
        session_id: Uuid,
    ) -> Result<Option<Arc<Session>>, SessionStorageError>;

    async fn get_session_by_token(
        &self,
        refresh_token: &[u8],
    ) -> Result<Option<Arc<Session>>, SessionStorageError>;
}

#[allow(private_bounds)]
pub struct SessionStorageImpl<Io: SessionStorageIo> {
    state: RwLock<State>,
    io: Io,
}

// TODO: too annoying, just write to the file and observe its changes
struct State {
    id_to_session: HashMap<Uuid, Arc<Session>>,
    name_to_sessions: HashMap<UsernameString, Vec<Arc<Session>>>,
    token_to_session: HashMap<Vec<u8>, Arc<Session>>,
}

impl From<SessionsData> for State {
    fn from(value: SessionsData) -> Self {
        let mut id_to_session = HashMap::new();
        let mut name_to_sessions: HashMap<_, Vec<Arc<Session>>> = HashMap::new();
        let mut token_to_session = HashMap::new();
        value.users
            .into_iter()
            .map(|user_data| {
                (
                    user_data.sessions
                        .into_iter()
                        .map(|session_data| {
                            Session {
                                session_id: session_data.session_id,
                                username: user_data.username.clone(),
                                refresh_token: session_data.refresh_token,
                                created_at: session_data.created_at,
                                expires_at: session_data.expires_at,
                            }
                        })
                        .map(Arc::new)
                        .collect::<Vec<_>>(),
                    user_data.username,
                )
            })
            .for_each(|(sessions, username)| {
                sessions.iter().for_each(|session| {
                    id_to_session.insert(session.session_id, session.clone());
                    token_to_session.insert(session.refresh_token.clone(), session.clone());
                });
                name_to_sessions.insert(username, sessions);
            });
        State {
            id_to_session,
            name_to_sessions,
            token_to_session,
        }
    }
}

#[allow(private_bounds)]
impl<Io: SessionStorageIo> SessionStorageImpl<Io> {
    async fn write_state(
        &self,
        mut state: impl DerefMut<Target=State>,
    ) -> Result<(), SessionStorageError> {
        let now = self.io.get_time();
        let mapped = SessionsData {
            users: state.name_to_sessions
                .iter()
                .filter_map(|(username, sessions)| {
                    let user_sessions: Vec<_> = sessions
                        .iter()
                        .filter_map(|session| {
                            if session.expires_at <= now {
                                None
                            } else {
                                Some(Self::session_to_session_data(session))
                            }
                        })
                        .collect();
                    let user_sessions = if user_sessions.is_empty() {
                        vec![
                            sessions.iter().max_by_key(|v| v.expires_at)
                                .map(|s| Self::session_to_session_data(s))
                                .expect("No user session"),
                        ]
                    } else {
                        user_sessions
                    };

                    Some(user_sessions)
                        .filter(|v| !v.is_empty())
                        .map(|v| {
                            UserSessionsData {
                                username: username.clone(),
                                sessions: v,
                            }
                        })
                })
                .collect()
        };
        self.io.write_session_file(&mapped).await?;

        // not really optimized, will be deleted by moving to observing
        // the session file
        let new_state = State::from(mapped);
        state.id_to_session = new_state.id_to_session;
        state.token_to_session = new_state.token_to_session;
        state.name_to_sessions = new_state.name_to_sessions;

        Ok(())
    }

    fn session_to_session_data(session: &Session) -> UserSessionData {
        UserSessionData {
            session_id: session.session_id,
            refresh_token: session.refresh_token.clone(),
            created_at: session.created_at,
            expires_at: session.expires_at,
        }
    }
}

#[async_trait]
impl<Io: SessionStorageIo> SessionStorage for SessionStorageImpl<Io> {
    async fn create_session(
        &self,
        username: &UsernameStr,
        created_at: OffsetDateTime,
        expires_at: OffsetDateTime,
    ) -> Result<Session, SessionStorageError> {
        let token = self.io.gen_refresh_token();
        let mut state = self.state.write().await;
        let new_session = Session {
            session_id: self.io.generate_uuid(),
            username: username.to_owned(),
            refresh_token: token.clone(),
            created_at,
            expires_at,
        };
        let new_session_arc = Arc::new(new_session.clone());
        match state.name_to_sessions.get_mut(username) {
            Some(sessions) => sessions.push(new_session_arc.clone()),
            None => {
                state.name_to_sessions.insert(
                    username.to_owned(),
                    vec![new_session_arc.clone()],
                );
            },
        };
        state.id_to_session.insert(new_session.session_id, new_session_arc.clone());
        state.token_to_session.insert(token, new_session_arc);
        self.write_state(state).await?;
        Ok(new_session)
    }

    async fn refresh_session(
        &self,
        refresh_token: &[u8],
        expires_at: OffsetDateTime,
    ) -> Result<Session, SessionStorageError> {
        let new_refresh_token = self.io.gen_refresh_token();
        let mut state = self.state.write().await;
        let session = state.token_to_session
            .get(refresh_token)
            .ok_or(SessionStorageError::SessionNotFound)?;
        let new_session = Session {
            session_id: session.session_id,
            username: session.username.clone(),
            refresh_token: new_refresh_token.clone(),
            created_at: session.created_at,
            expires_at,
        };
        let new_session_arc = Arc::new(new_session.clone());
        state.id_to_session.insert(new_session.session_id, new_session_arc.clone());
        let name_to_sessions = state.name_to_sessions
            .get_mut(&new_session.username)
            .expect("Session cache incoherent");
        let session_index = name_to_sessions
            .iter()
            .position(|s| s.refresh_token == refresh_token)
            .expect("Session cache incoherent");
        name_to_sessions[session_index] = new_session_arc.clone();
        state.token_to_session.remove(refresh_token);
        state.token_to_session.insert(new_refresh_token, new_session_arc);
        self.write_state(state).await?;
        Ok(new_session)
    }

    async fn delete_session(
        &self,
        session_id: Uuid,
    ) -> Result<bool, SessionStorageError> {
        let mut state = self.state.write().await;
        match state.id_to_session.remove(&session_id) {
            Some(session) => {
                let was_removed = state.name_to_sessions
                    .remove(&session.username)
                    .is_some();
                assert!(was_removed);

                let was_removed = state.token_to_session
                    .remove(&session.refresh_token)
                    .is_some();
                assert!(was_removed);

                self.write_state(state).await?;
                Ok(true)
            },
            None => Ok(false)
        }
    }

    async fn get_session_by_id(
        &self,
        session_id: Uuid,
    ) -> Result<Option<Arc<Session>>, SessionStorageError> {
        Ok(
            self.state
                .read()
                .await
                .id_to_session
                .get(&session_id)
                .cloned(),
        )
    }

    async fn get_session_by_token(
        &self,
        refresh_token: &[u8],
    ) -> Result<Option<Arc<Session>>, SessionStorageError> {
        Ok(
            self.state
                .read()
                .await
                .token_to_session
                .get(refresh_token)
                .cloned(),
        )
    }
}

pub type ProductionSessionStorage = SessionStorageImpl<ProductionSessionStorageIo>;

impl ProductionSessionStorage {
    pub async fn new(
        app_config: &AppConfig,
    ) -> Result<ProductionSessionStorage, SessionStorageError> {
        let mut path = app_config.data_directory.to_path_buf();
        path.push(SESSION_STORAGE_PATH);
        let io = ProductionSessionStorageIo::new(&path).await?;
        let state: State = io.read_session_file()
            .await?
            .into();
        Ok(
            SessionStorageImpl {
                state: RwLock::new(state),
                io,
            }
        )
    }
}
