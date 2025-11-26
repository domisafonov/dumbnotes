use time::OffsetDateTime;
use uuid::Uuid;
use crate::username_string::UsernameString;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Session {
    pub session_id: Uuid,
    pub username: UsernameString,
    pub refresh_token: Vec<u8>,
    pub created_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
}
