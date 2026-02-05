use uuid::Uuid;
use data::UsernameString;

pub struct LoginResult {
    pub refresh_token: Vec<u8>,
    pub access_token: String,
}

#[derive(Debug)]
pub enum SessionInfo {
    Valid(KnownSession),
    Expired(KnownSession),
}

#[derive(Debug)]
pub struct KnownSession {
    pub session_id: Uuid,
    pub username: UsernameString,
}
