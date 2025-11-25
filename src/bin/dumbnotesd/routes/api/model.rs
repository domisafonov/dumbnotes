use dumbnotes::data::{Note, NoteInfo};
use dumbnotes::username_string::UsernameString;

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
