use dumbnotes::ipc::caller::CallerError;
use protobuf_common::ProtobufRequestError;
use storage_ipc_data::bindings::StorageError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageAccessorError {
    #[error("data size is over the set limits")]
    TooBig,

    #[error("note not found")]
    NotFound,

    #[error("calling the storage daemon failed: {0}")]
    Caller(#[from] CallerError),

    #[error("storage daemon internal error")]
    StorageDaemonInternalError,

    #[error(transparent)]
    ProtobufError(#[from] ProtobufRequestError),

    #[error("invalid credentials")]
    InvalidCredentials
}

impl From<StorageError> for StorageAccessorError {
    fn from(value: StorageError) -> Self {
        match value {
            StorageError::InternalError => StorageAccessorError::StorageDaemonInternalError,
            StorageError::TooBig => StorageAccessorError::TooBig,
            StorageError::NotFound => StorageAccessorError::NotFound,
            StorageError::InvalidCredentials => StorageAccessorError::InvalidCredentials,
        }
    }
}
