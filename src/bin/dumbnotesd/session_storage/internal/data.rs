use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use dumbnotes::username_string::UsernameString;

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SessionsData {
    #[serde(rename = "user")]
    pub users: Vec<UserSessionsData>,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UserSessionsData {
    pub username: UsernameString,

    #[serde(rename = "session")]
    pub sessions: Vec<UserSessionData>,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct UserSessionData {
    #[serde(with = "dumbnotes::serde::base64vec")]
    pub refresh_token: Vec<u8>,

    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
}
