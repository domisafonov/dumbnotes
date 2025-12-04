use crate::file_watcher::{Event, FileWatchGuard, FileWatcher, FileWatcherError, ProductionFileWatcher};
use async_trait::async_trait;
use dumbnotes::username_string::UsernameStr;
use futures::StreamExt;
use log::{debug, error, info, trace};
use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::{oneshot, RwLock, RwLockWriteGuard};
use crate::user_db::internal::data::UsersData;
use crate::user_db::internal::user::User;
use crate::user_db::UserDbError;

#[async_trait]
pub trait UserDbIo: Send + Sync {
    async fn get_user(
        &self,
        username: &UsernameStr,
    ) -> Result<Option<User>, UserDbError>;
}

pub struct ProductionUserDbIo {
    die_notice: ManuallyDrop<oneshot::Sender<()>>,
    users: Arc<RwLock<HashMap<String, User>>>,
}

impl Drop for ProductionUserDbIo {
    fn drop(&mut self) {
        let _ = unsafe { ManuallyDrop::take(&mut self.die_notice) }
            .send(());
    }
}

impl ProductionUserDbIo {
    pub async fn new(
        user_db_path: impl AsRef<Path> + Send,
        file_watcher: ProductionFileWatcher,
    ) -> Result<Self, UserDbError> {
        trace!("creating user storage");

        let user_db_path = user_db_path.as_ref().to_owned();
        debug!("reading user db at \"{}\"", user_db_path.display());
        let data = Self::read_data(&user_db_path).await?;

        let users = Arc::new(RwLock::new(data));
        let (die_notice_sender, die_notice_receiver) = oneshot::channel();
        let file_watch_guard = file_watcher.watch(&user_db_path)?;
        tokio::spawn(
            Self::file_updates_watcher(
                user_db_path,
                users.clone(),
                die_notice_receiver,
                file_watch_guard,
            )
        );

        Ok(
            ProductionUserDbIo {
                die_notice: ManuallyDrop::new(die_notice_sender),
                users,
            }
        )
    }

    async fn file_updates_watcher(
        user_db_path: PathBuf,
        users: Arc<RwLock<HashMap<String, User>>>,
        mut die_notice: oneshot::Receiver<()>,
        file_watch_guard: <ProductionFileWatcher as FileWatcher>::Guard,
    ) {
        trace!("watching user db updates at \"{}\"", user_db_path.display());
        let mut events = pin!(file_watch_guard.get_events());
        loop {
            tokio::select! {
                _ = &mut die_notice => break,
                maybe_event = events.next() => match maybe_event.expect("file event stream finished") {
                    Err(e) => match e {
                        FileWatcherError::Overflow(_) => Self
                            ::read_and_replace(
                                users.write().await,
                                &user_db_path
                            )
                            .await,
                        _ => {
                            error!(
                                "failed to watch user db updates at \"{}\"",
                                user_db_path.display(),
                            );
                        }
                    },
                    Ok(Event::Any) => Self
                        ::read_and_replace(
                            users.write().await,
                            &user_db_path,
                        )
                        .await,
                },
            }
        }
        trace!(
            "stopped observing user db updates at {}",
            user_db_path.display(),
        );
    }

    async fn read_and_replace(
        mut users: RwLockWriteGuard<'_, HashMap<String, User>>,
        user_db_path: &Path,
    ) {
        info!("reading updated user db at \"{}\"", user_db_path.display());
        let data = match Self::read_data(user_db_path).await {
            Ok(d) => d,
            Err(e) => {
                error!(
                    "failed to read user db at \"{}\": {e}",
                    user_db_path.display(),
                );
                return;
            }
        };
        *users = data;
    }

    async fn read_data(
        user_db_path: &Path,
    ) -> Result<HashMap<String, User>, UserDbError> {
        trace!("reading user db at \"{}\"", user_db_path.display());
        let db_str = fs::read_to_string(&user_db_path).await?;
        trace!("read user db data at \"{}\": {db_str}", user_db_path.display());
        let parsed = toml::from_str::<UsersData>(&db_str)?;
        trace!("parsed user db data at \"{}\": {parsed:?}", user_db_path.display());
        Ok(
            HashMap::from_iter(
                parsed.users
                    .into_iter()
                    .map(|u|
                        (u.username.clone(), u.into())
                    )
            )
        )
    }
}

#[async_trait]
impl UserDbIo for ProductionUserDbIo {
    async fn get_user(
        &self,
        username: &UsernameStr,
    ) -> Result<Option<User>, UserDbError> {
        Ok(self.users.read().await.get::<str>(username).cloned())
    }
}
