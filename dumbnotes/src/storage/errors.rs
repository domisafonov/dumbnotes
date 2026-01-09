use thiserror::Error;
use time::error::ComponentRange;
use tokio::io::Error as IoError;
use unix::errors::CheckAccessError;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("data directory is not initialized properly")]
    DataDirNotInitialized,

    #[error(transparent)]
    Io(#[from] IoError),

    #[error("insufficient permissions to access storage")]
    Permission,

    #[error("file too large")]
    TooBig,
    
    #[error("cannot interpret timestamp")]
    Timestamp(#[from] ComponentRange),
    
    #[error("note not found")]
    NoteNotFound,

    #[error(transparent)]
    CheckAccessError(CheckAccessError),
}
