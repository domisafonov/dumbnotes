use std::path::Path;
use std::sync::Arc;
use async_trait::async_trait;
use log::trace;
use tokio::task::spawn_blocking;
use dumbnotes::hasher::{Hasher, ProductionHasher};
use dumbnotes::nix::check_secret_file_ro_access;
use dumbnotes::username_string::UsernameStr;
use crate::file_watcher::ProductionFileWatcher;
use crate::user_db::internal::io_trait::{ProductionUserDbIo, UserDbIo};
use crate::user_db::UserDbError;

mod io_trait;
#[cfg(test)] mod tests;
mod data;
mod user;

#[async_trait]
pub trait UserDb: Send + Sync {
    async fn check_user_credentials(
        &self,
        username: &UsernameStr,
        password: &str,
    ) -> Result<bool, UserDbError>;
}

#[allow(private_bounds)]
pub struct UserDbImpl<H: Hasher + 'static, Io: UserDbIo> {
    hasher: Arc<H>,
    io: Io,
}

#[async_trait]
impl<H: Hasher, Io: UserDbIo> UserDb for UserDbImpl<H, Io> {
    async fn check_user_credentials(
        &self,
        username: &UsernameStr,
        password: &str,
    ) -> Result<bool, UserDbError> {
        let user = self.io
            .get_user(username)
            .await?;
        trace!("checking credentials for \"{username}\"");
        match user {
            None => {
                trace!("user \"{username}\" not authenticated");
                Ok(false)
            },
            Some(user) => {
                trace!("user \"{username}\" correctly authenticated");
                let hasher = self.hasher.clone();
                let password = password.to_string(); // TODO: erase allocations containing passwords on drop everywhere
                Ok(
                    spawn_blocking(move ||
                        hasher
                            .check_hash(
                                user.hash.password_hash(),
                                &password
                            )
                    ).await.unwrap()
                )
            }
        }
    }
}

pub type ProductionUserDb = UserDbImpl<ProductionHasher, ProductionUserDbIo>;

impl ProductionUserDb {
    pub async fn new(
        user_db_path: &Path,
        hasher: ProductionHasher,
        file_watcher: ProductionFileWatcher,
    ) -> Result<ProductionUserDb, UserDbError> {
        check_secret_file_ro_access(user_db_path)?;
        Ok(
            UserDbImpl {
                hasher: Arc::new(hasher),
                io: ProductionUserDbIo::new(
                    user_db_path,
                    file_watcher,
                ).await?,
            }
        )
    }
}
