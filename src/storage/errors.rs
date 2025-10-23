use thiserror::Error;

use tokio::io::Error as IoError;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("user directory does not exist")]
    DoesNotExist,

    #[error(transparent)]
    IoError(#[from] IoError),

    #[error("insufficient permissions to access storage")]
    PermissionError,

    #[error("file too large")]
    TooBigError,
}
