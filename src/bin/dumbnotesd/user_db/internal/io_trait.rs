use crate::user_db::internal::data::UsersData;
use crate::user_db::internal::user::User;
use crate::user_db::UserDbError;
use async_trait::async_trait;
use dumbnotes::username_string::UsernameStr;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::sync::Arc;
use futures::StreamExt;
use tokio::fs;
use tokio::sync::{Notify, RwLock, RwLockWriteGuard};
use crate::file_watcher::{Event, FileWatchGuard, FileWatcher, FileWatcherError, ProductionFileWatcher};

#[async_trait]
pub trait UserDbIo: Send + Sync {
    async fn get_user(
        &self,
        username: &UsernameStr,
    ) -> Result<Option<User>, UserDbError>;
}

pub struct ProductionUserDbIo {
    users: Arc<RwLock<HashMap<String, User>>>,
    die_notice: Arc<Notify>,
}

impl Drop for ProductionUserDbIo {
    fn drop(&mut self) {
        self.die_notice.notify_one()
    }
}

impl ProductionUserDbIo {
    pub async fn new(
        user_db_path: impl AsRef<Path> + Send,
        file_watcher: ProductionFileWatcher,
    ) -> Result<Self, UserDbError> {
        let user_db_path = user_db_path.as_ref().to_owned();
        let data = Self::read_data(&user_db_path).await?;

        let users = Arc::new(RwLock::new(data));
        let die_notice = Arc::new(Notify::new());
        let file_watch_guard = file_watcher.watch(&user_db_path)?;
        tokio::spawn(
            Self::file_updates_watcher(
                user_db_path,
                users.clone(),
                die_notice.clone(),
                file_watch_guard,
            )
        );

        Ok(
            ProductionUserDbIo {
                users,
                die_notice,
            }
        )
    }

    async fn file_updates_watcher(
        user_db_path: PathBuf,
        users: Arc<RwLock<HashMap<String, User>>>,
        die_notice: Arc<Notify>,
        file_watch_guard: <ProductionFileWatcher as FileWatcher>::Guard,
    ) {
        let mut events = pin!(file_watch_guard.get_events());
        loop {
            let _ = tokio::select! {
                    _ = die_notice.notified() => break,
                    maybe_event = events.next() => match maybe_event.expect("file event stream finished") {
                        Err(e) => match(e) {
                            FileWatcherError::Overflow(_) => Self
                                ::read_and_replace(
                                    users.write().await,
                                    &user_db_path
                                )
                                .await,
                            _ => {
                                // TODO: log
                                Ok(())
                            }
                        },
                        Ok(Event::Any) => Self
                            ::read_and_replace(
                                users.write().await,
                                &user_db_path,
                            )
                            .await,
                    },
                };
        }
    }

    async fn read_and_replace(
        mut users: RwLockWriteGuard<'_, HashMap<String, User>>,
        user_db_path: &Path,
    ) -> Result<(), UserDbError> {
        let data = match Self::read_data(user_db_path).await {
            Ok(d) => d,
            Err(e) => {
                // TODO: log the error
                return Ok(())
            }
        };
        *users = data;
        Ok(())
    }

    async fn read_data(
        user_db_path: &Path,
    ) -> Result<HashMap<String, User>, UserDbError> {
        let db_str = fs::read_to_string(&user_db_path).await?;
        let parsed = toml::from_str::<UsersData>(&db_str)?;
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
