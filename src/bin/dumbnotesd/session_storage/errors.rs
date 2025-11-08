use std::io::Error as IoError;
use thiserror::Error;
use crate::file_watcher::FileWatcherError;

#[derive(Debug, Error)]
pub enum SessionStorageError {
    #[error(transparent)]
    Io(#[from] IoError),

    #[error(transparent)]
    LockingFailed(std::fs::TryLockError),

    #[error("invalid session file contents: {0}")]
    Parsing(#[from] toml::de::Error),

    #[error("serializing the session info failed: {0}")]
    Serialization(#[from] toml::ser::Error),

    #[error("session not found")]
    SessionNotFound,

    #[error("failed to watch session file: {0}")]
    SessionFileWatch(#[from] FileWatcherError),
}

impl From<std::fs::TryLockError> for SessionStorageError {
    fn from(e: std::fs::TryLockError) -> Self {
        match e {
            std::fs::TryLockError::WouldBlock => SessionStorageError::LockingFailed(e),
            std::fs::TryLockError::Error(e) => SessionStorageError::Io(e),
        }
    }
}
