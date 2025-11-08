use crate::app_constants::SESSION_STORAGE_PATH;
use crate::session_storage::internal::data::{SessionsData, UserSessionData, UserSessionsData};
use crate::session_storage::internal::io_trait::{ProductionSessionStorageIo, SessionStorageIo};
use crate::session_storage::internal::session::Session;
use crate::session_storage::SessionStorageError;
use async_trait::async_trait;
use dumbnotes::config::app_config::AppConfig;
use dumbnotes::username_string::{UsernameStr, UsernameString};
use std::collections::HashMap;
use std::ops::DerefMut;
use std::path::Path;
use std::pin::pin;
use std::sync::Arc;
use futures::{select_biased, FutureExt, StreamExt};
use time::{Duration, OffsetDateTime};
use tokio::spawn;
use tokio::sync::{Notify, RwLock};
use uuid::Uuid;
use crate::file_watcher::{Event, FileWatchGuard, FileWatcher, FileWatcherError, ProductionFileWatcher};

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
pub struct SessionStorageImpl<Io: SessionStorageIo, W: FileWatcher> {
    state: Arc<RwLock<State>>,
    io: Arc<Io>,
    file_watch_guard: W::Guard,
    die_notice: Arc<Notify>,
}

impl<Io: SessionStorageIo, W: FileWatcher> Drop for SessionStorageImpl<Io, W> {
    fn drop(&mut self) {
        self.die_notice.notify_one()
    }
}

struct State {
    id_to_session: HashMap<Uuid, Arc<Session>>,
    name_to_sessions_cache: HashMap<UsernameString, Vec<Arc<Session>>>,
    token_to_session_cache: HashMap<Vec<u8>, Arc<Session>>,
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
            name_to_sessions_cache: name_to_sessions,
            token_to_session_cache: token_to_session,
        }
    }
}

#[allow(private_bounds)]
impl<Io: SessionStorageIo, W: FileWatcher> SessionStorageImpl<Io, W> {
    pub async fn new_impl(
        path: &Path,
        io: Arc<Io>,
        file_watcher: W,
        factory: impl FnOnce(
            Arc<RwLock<State>>,
            W::Guard,
            Arc<Notify>,
        ) -> Self,
    ) -> Result<Self, SessionStorageError> {
        let state: State = io.read_session_file()
            .await?
            .into();
        let file_watch_guard = file_watcher.watch(path)?;
        let events = file_watch_guard.get_events().fuse();
        let die_notice = Arc::new(Notify::new());
        let die = die_notice.clone();
        let state = Arc::new(RwLock::new(state));
        let s = state.clone();
        let m_io = io.clone();
        spawn(async move {
            let mut die_notice = pin!(die.notified().fuse());
            let mut file_events = pin!(events);
            loop {
                // TODO: log errors
                let _ = select_biased! {
                    _ = die_notice => break,
                    maybe_event = file_events.next().fuse() => match maybe_event.expect("file event stream finished") {
                        Err(e) => match e {
                            FileWatcherError::Overflow(_) => Self::read_and_replace(&m_io, &s).await,
                            _ => {
                                // TODO: log
                                Ok(())
                            },
                        },
                        Ok(Event::Any) => Self::read_and_replace(&m_io, &s).await,
                    },
                };
            }
        });
        Ok(
            factory(
                state,
                file_watch_guard,
                die_notice,
            )
        )
    }

    async fn read_and_replace(
        io: &Io,
        state: &RwLock<State>,
    ) -> Result<(), SessionStorageError> {
        todo!()
    }

    async fn write_state(
        &self,
        state: impl DerefMut<Target=State>,
    ) -> Result<(), SessionStorageError> {
        let now = self.io.get_time();
        let mapped = SessionsData {
            users: state.name_to_sessions_cache
                .iter()
                .filter_map(|(username, sessions)| {
                    let user_sessions: Vec<_> = sessions
                        .iter()
                        .filter_map(|session| {
                            if session.expires_at + Duration::weeks(5) <= now { // TODO: find and order/properly delegate all time-related stuff
                                None
                            } else {
                                Some(Self::session_to_session_data(session))
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
        self.io.write_session_file(&mapped).await?;
        self.file_watch_guard.trigger_modification();
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
impl<Io: SessionStorageIo, W: FileWatcher> SessionStorage for SessionStorageImpl<Io, W> {
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
        match state.name_to_sessions_cache.get_mut(username) {
            Some(sessions) => sessions.push(new_session_arc),
            None => {
                state.name_to_sessions_cache.insert(
                    username.to_owned(),
                    vec![new_session_arc],
                );
            },
        };
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
        let session = state.token_to_session_cache
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
        let name_to_sessions = state.name_to_sessions_cache
            .get_mut(&new_session.username)
            .expect("Session cache incoherent");
        let session_index = name_to_sessions
            .iter()
            .position(|s| s.refresh_token == refresh_token)
            .expect("Session cache incoherent");
        name_to_sessions[session_index] = new_session_arc.clone();
        self.write_state(state).await?;
        Ok(new_session)
    }

    async fn delete_session(
        &self,
        session_id: Uuid,
    ) -> Result<bool, SessionStorageError> {
        let mut state = self.state.write().await;
        let found_username = state.id_to_session
            .get(&session_id)
            .map(|s| s.username.clone());
        match found_username {
            Some(found_username) => {
                let (_, users_sessions) = state.name_to_sessions_cache
                    .iter_mut()
                    .find(|(username, _)| **username == found_username)
                    .expect("Session cache incoherent");
                users_sessions
                    .remove(
                        users_sessions.iter()
                            .position(|s| s.session_id == session_id)
                            .expect("Session cache incoherent")
                    );

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
                .token_to_session_cache
                .get(refresh_token)
                .cloned(),
        )
    }
}

pub type ProductionSessionStorage = SessionStorageImpl<
    ProductionSessionStorageIo,
    ProductionFileWatcher,
>;

impl ProductionSessionStorage {
    pub async fn new(
        app_config: &AppConfig,
        file_watcher: ProductionFileWatcher,
    ) -> Result<ProductionSessionStorage, SessionStorageError> {
        let mut path = app_config.data_directory.to_path_buf();
        path.push(SESSION_STORAGE_PATH);
        let io = Arc::new(ProductionSessionStorageIo::new(&path).await?);
        let io2 = io.clone();
        SessionStorageImpl
            ::new_impl(
                &path,
                io2,
                file_watcher,
                move |state, file_watch_guard, die_notice| {
                    SessionStorageImpl {
                        state,
                        io,
                        file_watch_guard,
                        die_notice,
                    }
                },
            )
            .await
    }
}
