use std::collections::HashMap;
use std::path::Path;
use async_trait::async_trait;
use tokio::fs;
use crate::config::UsernameString;
use crate::user_db::internal::data::UsersData;
use crate::user_db::internal::user::User;
use crate::user_db::UserDbError;

#[async_trait]
pub(super) trait UserDbIo: Send + Sync {
    async fn get_user(
        &self,
        username: &UsernameString,
    ) -> Result<Option<User>, UserDbError>;
}

pub struct ProductionUserDbIo {
    users: HashMap<String, User>,
}

impl ProductionUserDbIo {
    pub async fn new(
        user_db_filename: impl AsRef<Path> + Send,
    ) -> Result<Self, UserDbError> {
        let db_str = fs::read_to_string(user_db_filename).await?;
        let mut parsed = toml::from_str::<UsersData>(&db_str)?;
        Ok(
            ProductionUserDbIo {
                users: HashMap::from_iter(
                parsed.users
                    .drain(..)
                    .map(|u|
                        (u.username.clone(), u.into())
                    )
                ),
            }
        )
    }
}

#[async_trait]
impl UserDbIo for ProductionUserDbIo {
    async fn get_user(
        &self,
        username: &UsernameString,
    ) -> Result<Option<User>, UserDbError> {
        Ok(self.users.get::<str>(username).cloned())
    }
}
