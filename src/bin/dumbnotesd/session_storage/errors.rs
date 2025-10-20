use std::fmt;
use std::fmt::Formatter;
use std::io::Error as IoError;

#[derive(Debug)]
pub enum SessionStorageError {
    IoError(IoError),
    LockingFailed(std::fs::TryLockError),
    ParsingError {
        message: String,
    },
    SerializationError {
        message: String,
    },
    SessionNotFound,
}

impl fmt::Display for SessionStorageError { // TODO: prettier strings
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SessionStorageError::IoError(_) => fmt::Debug::fmt(self, f),
            SessionStorageError::LockingFailed(_) => fmt::Debug::fmt(self, f),
            SessionStorageError::ParsingError { message } =>
                f.write_fmt(format_args!("Session db parsing error: {}", message)),
            SessionStorageError::SerializationError { message } =>
                f.write_fmt(format_args!("Session db serialization error: {}", message)),
            SessionStorageError::SessionNotFound =>
                f.write_str("Session not found"),
        }
    }
}
impl std::error::Error for SessionStorageError {}

impl From<IoError> for SessionStorageError {
    fn from(e: IoError) -> Self {
        Self::IoError(e)
    }
}

impl From<toml::de::Error> for SessionStorageError {
    fn from(e: toml::de::Error) -> Self {
        SessionStorageError::ParsingError {
            message: format!("{e}"),
        }
    }
}

impl From<toml::ser::Error> for SessionStorageError {
    fn from(e: toml::ser::Error) -> Self {
        SessionStorageError::SerializationError {
            message: format!("{e}"),
        }
    }
}

impl From<std::fs::TryLockError> for SessionStorageError {
    fn from(e: std::fs::TryLockError) -> Self {
        match e {
            std::fs::TryLockError::WouldBlock => SessionStorageError::LockingFailed(e),
            std::fs::TryLockError::Error(e) => SessionStorageError::IoError(e),
        }
    }
}
