use async_trait::async_trait;
use dumbnotes::config::app_config::AppConfig;
use dumbnotes::storage::StorageError;
use dumbnotes::username_string::UsernameStr;
use crate::session_storage::internal::io_trait::{ProductionSessionStorageIo, SessionStorageIo};
use crate::session_storage::internal::session::Session;
use crate::session_storage::SessionStorageError;

#[cfg(test)] mod tests;
mod data;
mod session;
mod io_trait;

#[async_trait]
trait SessionStorage: Send + Sync {
    async fn create_session(
        &self,
        username: UsernameStr<'_>,
    ) -> Result<Session, StorageError>;

    async fn delete_session(
        &self,
        username: &UsernameStr<'_>,
    ) -> Result<(), StorageError>;

    async fn get_session(
        &self,
        refresh_token: &[u8],
    ) -> Result<Option<Session>, StorageError>;
}

#[allow(private_bounds)]
pub struct SessionStorageImpl<Io: SessionStorageIo> {
    io: Io,
}

#[async_trait]
impl<Io: SessionStorageIo> SessionStorage for SessionStorageImpl<Io> {
    async fn create_session(&self, username: UsernameStr<'_>) -> Result<Session, StorageError> {
        todo!()
    }

    async fn delete_session(&self, username: &UsernameStr<'_>) -> Result<(), StorageError> {
        todo!()
    }

    async fn get_session(&self, refresh_token: &[u8]) -> Result<Option<Session>, StorageError> {
        todo!()
    }
}

pub type ProductionSessionStorage = SessionStorageImpl<ProductionSessionStorageIo>;

impl ProductionSessionStorage {
    pub async fn new(
        app_config: &AppConfig,
    ) -> Result<ProductionSessionStorage, SessionStorageError> {
        todo!()
    }
}
