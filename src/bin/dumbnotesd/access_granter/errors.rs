use thiserror::Error;

#[derive(Debug, Error)]
pub enum AccessGranterError {
    #[error("token format error")]
    HeaderFormatError,

    #[error("invalid token")]
    InvalidToken,

    #[error("invalid credentials")]
    InvalidCredentials,
}
