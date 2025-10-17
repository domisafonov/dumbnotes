use crate::app_constants::SESSION_STORAGE_PATH;
use crate::session_storage::internal::data::{SessionsData, UserSessionData, UserSessionsData};
use crate::session_storage::internal::io_trait::{ProductionSessionStorageIo, SessionStorageIo};
use crate::session_storage::internal::session::Session;
use crate::session_storage::SessionStorageError;
use async_trait::async_trait;
use dumbnotes::config::app_config::AppConfig;
use dumbnotes::username_string::{UsernameStr, UsernameString};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use rand::rngs::StdRng;
use time::OffsetDateTime;
use tokio::sync::RwLock;
use dumbnotes::rng::SyncRng;

#[cfg(test)] mod tests;
mod data;
mod session;
mod io_trait;

#[async_trait]
trait SessionStorage: Send + Sync {
    async fn create_session(
        &self,
        username: &UsernameStr,
        expires_at: OffsetDateTime,
    ) -> Result<Session, SessionStorageError>;

    async fn delete_session(
        &self,
        refresh_token: &[u8],
    ) -> Result<bool, SessionStorageError>;

    async fn get_session(
        &self,
        refresh_token: &[u8],
    ) -> Result<Option<Arc<Session>>, SessionStorageError>;
}

#[allow(private_bounds)]
pub struct SessionStorageImpl<Io: SessionStorageIo> {
    state: RwLock<State>,
    io: Io,
}

struct State {
    name_to_sessions: HashMap<UsernameString, Vec<Arc<Session>>>,
    token_to_session: HashMap<Vec<u8>, Arc<Session>>,
}

impl From<SessionsData> for State {
    fn from(value: SessionsData) -> Self {
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
                                username: user_data.username.clone(),
                                refresh_token: session_data.refresh_token,
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
                    token_to_session.insert(session.refresh_token.clone(), session.clone());
                });
                name_to_sessions.insert(username, sessions);
            });
        State {
            name_to_sessions,
            token_to_session,
        }
    }
}

#[allow(private_bounds)]
impl<Io: SessionStorageIo> SessionStorageImpl<Io> {
    async fn write_state(
        &self,
        state: impl Deref<Target=State>,
    ) -> Result<(), SessionStorageError> {
        let now = self.io.get_time();
        let mapped = SessionsData {
            users: state.name_to_sessions
                .iter()
                .filter_map(|(username, sessions)| {
                    let user_sessions: Vec<_> = sessions
                        .iter()
                        .filter_map(|session| {
                            if session.expires_at >= now {
                                None
                            } else {
                                Some(
                                    UserSessionData {
                                        refresh_token: session.refresh_token.clone(),
                                        expires_at: session.expires_at,
                                    }
                                )
                            }
                        })
                        .collect();

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
        self.io.write_session_file(mapped).await
    }
}

#[async_trait]
impl<Io: SessionStorageIo> SessionStorage for SessionStorageImpl<Io> {
    async fn create_session(
        &self,
        username: &UsernameStr,
        expires_at: OffsetDateTime,
    ) -> Result<Session, SessionStorageError> {
        let token = self.io.gen_refresh_token();
        let mut state = self.state.write().await;
        let new_session = Session {
            username: username.to_owned(),
            refresh_token: token.clone(),
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
        state.token_to_session.insert(token, new_session_arc);
        self.write_state(state).await?;
        Ok(new_session)
    }

    async fn delete_session(
        &self,
        refresh_token: &[u8],
    ) -> Result<bool, SessionStorageError> {
        let mut state = self.state.write().await;
        match state.token_to_session.remove(refresh_token) {
            Some(session) => {
                let was_removed = state.name_to_sessions
                    .remove(&session.username)
                    .is_some();
                assert!(was_removed);
                self.write_state(state).await?;
                Ok(true)
            },
            None => Ok(false)
        }
    }

    async fn get_session(
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
        rng: SyncRng<StdRng>,
    ) -> Result<ProductionSessionStorage, SessionStorageError> {
        let mut path = app_config.user_db.to_path_buf();
        path.push(SESSION_STORAGE_PATH);
        let io = ProductionSessionStorageIo::new(
            &path,
            rng,
        ).await?;
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
