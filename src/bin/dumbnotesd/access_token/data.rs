use time::OffsetDateTime;
use uuid::Uuid;
use dumbnotes::username_string::UsernameString;

pub(super) const SESSION_ID_CLAIM_NAME: &str = "session_id";

pub struct AccessTokenData {
    pub session_id: Uuid,
    pub username: UsernameString,
    pub not_before: OffsetDateTime,
    pub expires_at: OffsetDateTime,
}
