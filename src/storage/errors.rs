use thiserror::Error;
use time::error::ComponentRange;
use tokio::io::Error as IoError;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("user directory does not exist")]
    UserDirDoesNotExist,

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
}
