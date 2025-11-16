use crate::user_db::internal::io_trait::{ProductionUserDbIo, UserDbIo};
use crate::user_db::UserDbError;
use async_trait::async_trait;
use log::trace;
use dumbnotes::config::app_config::AppConfig;
use dumbnotes::hasher::{Hasher, ProductionHasher};
use dumbnotes::username_string::UsernameStr;
use crate::file_watcher::ProductionFileWatcher;

mod io_trait;
#[cfg(test)] mod tests;
mod data;
mod user;

#[async_trait]
pub trait UserDb: Send + Sync {
    async fn does_user_exist(
        &self,
        username: &UsernameStr,
    ) -> Result<bool, UserDbError>;

    async fn check_user_credentials(
        &self,
        username: &UsernameStr,
        password: &str,
    ) -> Result<bool, UserDbError>;
}

#[allow(private_bounds)]
pub struct UserDbImpl<H: Hasher, Io: UserDbIo> {
    hasher: H,
    io: Io,
}

#[async_trait]
impl<H: Hasher, Io: UserDbIo> UserDb for UserDbImpl<H, Io> {
    async fn does_user_exist(
        &self,
        username: &UsernameStr,
    ) -> Result<bool, UserDbError> {
        trace!("checking user \"{username}\"");
        let does_exist = self.io
            .get_user(username)
            .await?
            .is_some();
        trace!("user \"{username}\" exists: {does_exist}");
        Ok(does_exist)
    }

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
                Ok(
                    self.hasher
                        .check_hash(
                            user.hash.password_hash(),
                            password
                        )
                )
            }
        }
    }
}

pub type ProductionUserDb = UserDbImpl<ProductionHasher, ProductionUserDbIo>;

impl ProductionUserDb {
    pub async fn new(
        app_config: &AppConfig,
        hasher: ProductionHasher,
        file_watcher: ProductionFileWatcher,
    ) -> Result<ProductionUserDb, UserDbError> {
        Ok(
            UserDbImpl {
                hasher,
                io: ProductionUserDbIo::new(
                    &app_config.user_db,
                    file_watcher,
                ).await?,
            }
        )
    }
}
