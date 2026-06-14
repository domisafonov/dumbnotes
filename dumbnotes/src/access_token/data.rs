use time::OffsetDateTime;
use uuid::Uuid;
use data::UsernameString;

#[derive(Debug, Clone)]
pub struct AccessTokenData {
    pub session_id: Uuid,
    pub username: UsernameString,
    pub not_before: OffsetDateTime,
    pub expires_at: OffsetDateTime,
}
