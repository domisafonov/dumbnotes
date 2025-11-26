mod internal;
mod errors;

use std::sync::Arc;
use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;
use crate::username_string::UsernameStr;

pub use errors::*;
pub use internal::ProductionSessionStorage;
pub use internal::session::Session;

#[async_trait]
pub trait SessionStorage: Send + Sync {
    async fn create_session(
        &self,
        username: &UsernameStr,
        created_at: OffsetDateTime,
        expires_at: OffsetDateTime,
    ) -> Result<Session, SessionStorageError>;

    async fn refresh_session(
        &self,
        refresh_token: &[u8],
        expires_at: OffsetDateTime,
    ) -> Result<Session, SessionStorageError>;

    async fn delete_session(
        &self,
        session_id: Uuid,
    ) -> Result<bool, SessionStorageError>;

    async fn get_session_by_id(
        &self,
        session_id: Uuid,
    ) -> Result<Option<Arc<Session>>, SessionStorageError>;

    async fn get_session_by_token(
        &self,
        refresh_token: &[u8],
    ) -> Result<Option<Arc<Session>>, SessionStorageError>;
}
