use async_trait::async_trait;
use crate::config::AppConfig;
use crate::hasher::{Hasher, ProductionHasher};
use crate::user_db::internal::io_trait::{ProductionUserDbIo, UserDbIo};
use crate::user_db::UserDbError;
use crate::username_string::UsernameString;

mod io_trait;
#[cfg(test)] mod tests;
mod data;
mod user;

#[async_trait]
pub trait UserDb: Send + Sync {
    async fn does_user_exist(
        &self,
        username: &UsernameString,
    ) -> Result<bool, UserDbError>;

    async fn check_user_credentials(
        &self,
        username: &UsernameString,
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
        username: &UsernameString,
    ) -> Result<bool, UserDbError> {
        Ok(
            self.io
                .get_user(username)
                .await?
                .is_some()
        )
    }

    async fn check_user_credentials(
        &self,
        username: &UsernameString,
        password: &str,
    ) -> Result<bool, UserDbError> {
        let user = self.io
            .get_user(username)
            .await?;

        match user {
            None => Ok(false),
            Some(user) => {
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
    ) -> Result<ProductionUserDb, UserDbError> {
        Ok(
            UserDbImpl {
                hasher,
                io: ProductionUserDbIo::new(&app_config.user_db).await?,
            }
        )
    }
}
