use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use dumbnotes::username_string::UsernameString;
use crate::session_storage::internal::session::Session;

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SessionsData {
    #[serde(rename = "session")]
    pub sessions: Vec<SessionData>,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SessionData {
    pub username: UsernameString,

    #[serde(with = "dumbnotes::serde::base64vec")]
    pub refresh_token: Vec<u8>,

    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
}

impl From<SessionData> for Session {
    fn from(value: SessionData) -> Self {
        Session {
            username: value.username,
            refresh_token: value.refresh_token,
            expires_at: value.expires_at,
        }
    }
}
