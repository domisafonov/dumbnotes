use time::UtcDateTime;
use data::{Note, NoteInfo};
use data::UsernameString;

pub struct LoginRequest {
    pub username: UsernameString,
    pub secret: LoginRequestSecret,
}

pub enum LoginRequestSecret {
    Password(String),
    RefreshToken(Vec<u8>),
}

pub struct LoginResponse {
    pub refresh_token: Vec<u8>,
    pub access_token: String,
}

pub struct NoteListResponse {
    pub notes_info: Vec<NoteInfo>,
}

pub struct NoteResponse(pub Note);

pub struct NoteWriteRequest {
    pub mtime: UtcDateTime,
    pub name: Option<String>,
    pub contents: String,
}
