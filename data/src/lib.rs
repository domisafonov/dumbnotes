mod username_string;

use argon2::PasswordHash;
use kinded::Kinded;
use time::{OffsetDateTime, UtcDateTime};
use uuid::Uuid;

pub use username_string::*;

#[derive(Clone, Copy, Debug)]
pub struct NoteMetadata {
    pub id: Uuid,
    pub mtime: UtcDateTime,
}

#[derive(Clone, Debug)]
pub struct NoteInfo {
    pub metadata: NoteMetadata,
    pub name: Option<String>,
}

// TODO: data is always validated for MAX_NOTE_LEN
#[derive(Clone, Debug)]
pub struct Note {
    pub metadata: NoteMetadata,
    pub name: Option<String>,
    pub contents: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct User {
    pub username: String,
    pub hash: PasswordHash,
}

#[derive(Clone, Debug, Eq, Kinded, PartialEq)]
pub enum Session {
    Api(ApiSession),
    Web(WebSession),
}

impl Session {
    pub fn get_session_id(&self) -> Uuid {
        match self {
            Session::Api(ApiSession { session_id, .. }) => *session_id,
            Session::Web(WebSession { session_id, .. }) => *session_id,
        }
    }

    pub fn get_username(&self) -> UsernameString {
        match self {
            Session::Api(ApiSession { username, .. }) => username.clone(),
            Session::Web(WebSession { username, .. }) => username.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApiSession {
    pub session_id: Uuid,
    pub username: UsernameString,
    pub refresh_token: Vec<u8>,
    pub created_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WebSession {
    pub session_id: Uuid,
    pub username: UsernameString,
    pub xsrf_token: Vec<u8>,
    pub created_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
}
