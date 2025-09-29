use async_trait::async_trait;
use crate::config::{AppConfig, UsernameString};
use crate::user_db::internal::io_trait::{ProductionUserDbIo, UserDbIo};
use crate::user_db::UserDbError;

mod io_trait;
#[cfg(test)] mod tests;
mod data;

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
pub struct UserDbImpl<Io: UserDbIo> {
    io: Io,
}

#[async_trait]
impl<Io: UserDbIo> UserDb for UserDbImpl<Io> {
    async fn does_user_exist(
        &self,
        username: &UsernameString,
    ) -> Result<bool, UserDbError> {
        todo!()
    }

    async fn check_user_credentials(
        &self,
        username: &UsernameString,
        password: &str,
    ) -> Result<bool, UserDbError> {
        todo!()
    }
}

pub type ProductionUserDb = UserDbImpl<ProductionUserDbIo>;

impl ProductionUserDb {
    pub async fn new(
        app_config: &AppConfig,
    ) -> Result<ProductionUserDb, UserDbError> {
        Ok(
            UserDbImpl {
                io: ProductionUserDbIo::new(&app_config.user_db).await?,
            }
        )
    }
}
