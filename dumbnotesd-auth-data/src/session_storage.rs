use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;
use data::UsernameString;

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
#[serde(tag = "type")]
pub enum UserSessionData {
    #[serde(rename = "api")] Api {
        session_id: Uuid,

        #[serde(with = "crate::serde::base64_vec")]
        refresh_token: Vec<u8>,

        #[serde(with = "time::serde::rfc3339")]
        created_at: OffsetDateTime,

        #[serde(with = "time::serde::rfc3339")]
        expires_at: OffsetDateTime,
    },
    #[serde(rename = "web")] Web {
        session_id: Uuid,

        #[serde(with = "crate::serde::base64_vec")]
        xsrf_token: Vec<u8>,

        #[serde(with = "time::serde::rfc3339")]
        created_at: OffsetDateTime,

        #[serde(with = "time::serde::rfc3339")]
        expires_at: OffsetDateTime,
    }
}
