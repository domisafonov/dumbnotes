use async_trait::async_trait;
use ::data::{UsernameStr, UsernameString};
use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use futures::{Stream, StreamExt};
use log::{error, info, trace};
use time::OffsetDateTime;
use tokio::spawn;
use tokio::sync::{oneshot, RwLock, RwLockWriteGuard};
use uuid::Uuid;
use unix::check_secret_file_rw_access;
use crate::file_watcher::{FileWatchGuard, FileWatcher, ProductionFileWatcher};
use crate::file_watcher::Event;
use crate::file_watcher::FileWatcherError;
use crate::app_constants::{REFRESH_TOKEN_GC_TIME, SESSION_STORAGE_PATH};
use crate::session_storage::internal::data::{SessionsData, UserSessionData, UserSessionsData};
use crate::session_storage::internal::io_trait::{ProductionSessionStorageIo, SessionStorageIo};
use crate::session_storage::{Session, SessionStorage, SessionStorageError};

#[cfg(test)] mod tests;
mod data;
pub mod session;
mod io_trait;

#[allow(private_bounds)]
pub struct SessionStorageImpl<Io: SessionStorageIo, W: FileWatcher> {
    die_notice: ManuallyDrop<oneshot::Sender<()>>,
    state: Arc<RwLock<State>>,
    io: Arc<Io>,
    file_watch_guard: W::Guard,
}

impl<Io: SessionStorageIo, W: FileWatcher> Drop for SessionStorageImpl<Io, W> {
    fn drop(&mut self) {
        trace!("session storage dropped");
        let _ = unsafe { ManuallyDrop::take(&mut self.die_notice) }
            .send(());
    }
}

#[derive(Debug)]
struct State {
    id_to_session: HashMap<Uuid, Arc<Session>>,
    name_to_sessions_cache: HashMap<UsernameString, Vec<Arc<Session>>>,
    token_to_session_cache: HashMap<Vec<u8>, Arc<Session>>,
}

impl From<SessionsData> for State {
    fn from(value: SessionsData) -> Self {
        let mut id_to_session = HashMap::new();
        let mut name_to_sessions_cache: HashMap<_, Vec<Arc<Session>>> = HashMap::new();
        let mut token_to_session_cache = HashMap::new();
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
                    token_to_session_cache.insert(session.refresh_token.clone(), session.clone());
                });
                name_to_sessions_cache.insert(username, sessions);
            });
        State {
            id_to_session,
            name_to_sessions_cache,
            token_to_session_cache,
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
            oneshot::Sender<()>,
        ) -> Self,
    ) -> Result<Self, SessionStorageError> {
        let state: State = io.read_session_file()
            .await?
            .into();
        let file_watch_guard = file_watcher.watch(path)?;
        let (die_notice_sender, die_notice_receiver) = oneshot::channel();
        let state = Arc::new(RwLock::new(state));
        spawn(
            Self::file_updates_watcher(
                state.clone(),
                io.clone(),
                die_notice_receiver,
                file_watch_guard.get_events(),
            )
        );
        Ok(
            factory(
                state,
                file_watch_guard,
                die_notice_sender,
            )
        )
    }

    async fn file_updates_watcher(
        state: Arc<RwLock<State>>,
        io: Arc<Io>,
        mut die_notice: oneshot::Receiver<()>,
        events: impl Stream<Item=Result<Event, FileWatcherError>>,
    ) {
        trace!("watching session db updates");
        let mut events = Box::pin(events);
        loop {
            let _ = tokio::select! {
                biased;
                _ = &mut die_notice => break,
                maybe_event = events.next() => match maybe_event.expect("file event stream finished") {
                    Err(e) => match e {
                        FileWatcherError::Overflow(_) => Self
                            ::read_and_replace(
                                &io,
                                &mut state.write().await,
                            )
                            .await,
                        _ => {
                            error!("failed to watch session db updates");
                            Ok(())
                        },
                    },
                    Ok(Event::Any) => Self
                        ::read_and_replace(
                            &io,
                            &mut state.write().await,
                        )
                        .await,
                },
            }.inspect_err(|e| {
                error!("failed to read session db: {}", e);
            });
        }
        trace!("stopped observing session db updates")
    }

    async fn read_and_replace(
        io: &Io,
        state: &mut RwLockWriteGuard<'_, State>,
    ) -> Result<(), SessionStorageError> {
        info!("reading updated session db");
        let new_state: State = io
            .read_session_file()
            .await
            .inspect(|v| {
                trace!("read session db: {v:?}")
            })?
            .into();
        **state = new_state;
        Ok(())
    }

    async fn write_state(
        &self,
        mut guard: RwLockWriteGuard<'_, State>,
    ) -> Result<(), SessionStorageError> {
        trace!("writing updated session db");
        let now = self.io.get_time();
        let mapped = SessionsData {
            users: guard.name_to_sessions_cache
                .iter()
                .filter_map(|(username, sessions)| {
                    let user_sessions: Vec<_> = sessions
                        .iter()
                        .filter_map(|session| {
                            if session.expires_at + REFRESH_TOKEN_GC_TIME <= now {
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
        trace!("new session state: {mapped:?}");
        info!("saving session state");
        self.io.write_session_file(&mapped).await?;
        self.file_watch_guard.skip_modification();
        Self::read_and_replace(
            &self.io,
            &mut guard,
        ).await?;
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
        let session_id = self.io.generate_uuid();
        info!(
            "creating new user session {session_id} \
                for user \"{username}\", expires at {expires_at}"
        );
        let token = self.io.gen_refresh_token();
        let mut state = self.state.write().await;
        let new_session = Session {
            session_id,
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
        info!(
            "refreshing session {} for user \"{}\", expires at {expires_at}",
            session.session_id,
            session.username,
        );
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
            .expect("session cache incoherent");
        let session_index = name_to_sessions
            .iter()
            .position(|s| s.refresh_token == refresh_token)
            .expect("session cache incoherent");
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
        info!(
            "terminating session {session_id} for user {}",
            found_username.as_ref().map(UsernameString::as_str).unwrap_or("None")
        );
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
        data_directory: &Path,
        file_watcher: ProductionFileWatcher,
    ) -> Result<ProductionSessionStorage, SessionStorageError> {
        trace!("creating session storage at {}", data_directory.display());
        let path = Self::get_storage_path(data_directory);
        check_secret_file_rw_access(&path)?;
        let io = Arc::new(ProductionSessionStorageIo::new(&path).await?);
        let io2 = io.clone();
        SessionStorageImpl
            ::new_impl(
                &path,
                io2,
                file_watcher,
                move |state, file_watch_guard, die_notice| {
                    SessionStorageImpl {
                        die_notice: ManuallyDrop::new(die_notice),
                        state,
                        io,
                        file_watch_guard,
                    }
                },
            )
            .await
    }
    
    pub fn get_storage_path(data_directory: &Path) -> PathBuf {
        let mut path = data_directory.to_path_buf();
        path.push(SESSION_STORAGE_PATH);
        path
    }
}
