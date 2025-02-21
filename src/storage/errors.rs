use std::fmt;
use std::io::Error as IoError;
use std::io::ErrorKind;

#[derive(Debug)]
pub enum StorageError {
    DirectoryDoesNotExist,
    IoError(IoError),
    PermissionError,
    TooBigError,
}
impl fmt::Display for StorageError { // TODO: prettier strings
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}
impl std::error::Error for StorageError {}

impl From<IoError> for StorageError {
    fn from(value: IoError) -> Self {
        if value.kind() == ErrorKind::NotFound {
            StorageError::DirectoryDoesNotExist
        } else {
            StorageError::IoError(value)
        }
    }
}
