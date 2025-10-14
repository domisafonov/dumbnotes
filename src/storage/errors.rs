use std::fmt;
use time::error::ComponentRange;

use tokio::io::Error as IoError;
use tokio::io::ErrorKind;

#[derive(Debug)]
pub enum StorageError {
    DoesNotExist,
    IoError(IoError),
    PermissionError,
    TooBigError,
    OutOfRangeDate,
}
impl fmt::Display for StorageError { // TODO: prettier strings
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}
impl std::error::Error for StorageError {}

impl From<IoError> for StorageError {
    fn from(value: IoError) -> Self {
        if value.kind() == ErrorKind::NotFound {
            StorageError::DoesNotExist
        } else {
            StorageError::IoError(value)
        }
    }
}

impl From<ComponentRange> for StorageError {
    fn from(_value: ComponentRange) -> Self {
        StorageError::OutOfRangeDate
    }
}
