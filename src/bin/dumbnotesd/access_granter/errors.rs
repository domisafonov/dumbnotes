use thiserror::Error;
use dumbnotes::access_token::AccessTokenGeneratorError;
use dumbnotes::session_storage::SessionStorageError;
use dumbnotes::user_db::UserDbError;

#[derive(Debug, Error)]
pub enum AccessGranterError {
    #[error("token format error")]
    HeaderFormatError,

    #[error("invalid token")]
    InvalidToken,

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error(transparent)]
    SessionStorageError(SessionStorageError),

    #[error(transparent)]
    UserDbError(#[from] UserDbError),

    #[error(transparent)]
    AccessTokenGeneratorError(#[from] AccessTokenGeneratorError),
}

impl From<SessionStorageError> for AccessGranterError {
    fn from(e: SessionStorageError) -> Self {
        match e {
            SessionStorageError::SessionNotFound => AccessGranterError::InvalidCredentials,
            _ => AccessGranterError::SessionStorageError(e),
        }
    }
}
