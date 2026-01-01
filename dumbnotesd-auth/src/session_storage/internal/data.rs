use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;
use dumbnotes::username_string::UsernameString;

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionsData {
    #[serde(default, rename = "user")]
    pub users: Vec<UserSessionsData>,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UserSessionsData {
    pub username: UsernameString,

    #[serde(default, rename = "session")]
    pub sessions: Vec<UserSessionData>,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UserSessionData {
    pub session_id: Uuid,

    #[serde(with = "dumbnotes::serde::base64_vec")]
    pub refresh_token: Vec<u8>,

    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,

    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
}
