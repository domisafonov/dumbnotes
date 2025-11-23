use dumbnotes::data::NoteInfo;
use dumbnotes::username_string::UsernameString;

#[derive(Clone, Eq, PartialEq)]
pub struct LoginRequest {
    pub username: UsernameString,
    pub secret: LoginRequestSecret,
}

#[derive(Clone, Eq, PartialEq)]
pub enum LoginRequestSecret {
    Password(String),
    RefreshToken(Vec<u8>),
}

#[derive(Clone, Eq, PartialEq)]
pub struct LoginResponse {
    pub refresh_token: Vec<u8>,
    pub access_token: String,
}

pub struct NoteListResponse {
    pub notes_info: Vec<NoteInfo>,
}
