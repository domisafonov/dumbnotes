use crate::user_db::internal::data::UsersData;
use crate::user_db::internal::user::User;
use crate::user_db::UserDbError;
use async_trait::async_trait;
use dumbnotes::username_string::UsernameStr;
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

#[async_trait]
pub trait UserDbIo: Send + Sync {
    async fn get_user(
        &self,
        username: &UsernameStr,
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
        let parsed = toml::from_str::<UsersData>(&db_str)?;
        Ok(
            ProductionUserDbIo {
                users: HashMap::from_iter(
                parsed.users
                    .into_iter()
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
        username: &UsernameStr,
    ) -> Result<Option<User>, UserDbError> {
        Ok(self.users.get::<str>(username).cloned())
    }
}
