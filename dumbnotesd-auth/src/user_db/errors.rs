use std::io::Error as IoError;
use thiserror::Error;
use dumbnotes::hasher::HasherError;
use dumbnotes::nix::CheckAccessError;
use crate::file_watcher::FileWatcherError;

#[derive(Debug, Error)]
pub enum UserDbError {
    #[error(transparent)]
    Io(#[from] IoError),

    #[error("invalid user db file contents: $0")]
    Parsing(#[from] toml::de::Error),
    
    #[error("failed to watch the db file: $0")]
    Watch(#[from] FileWatcherError),
    
    #[error(transparent)]
    CheckAccess(#[from] CheckAccessError),
    
    #[error("hashing failed: {0}")]
    Hashing(#[from] HasherError),
}
