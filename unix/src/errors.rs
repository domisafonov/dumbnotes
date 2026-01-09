use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CheckAccessError {
    #[error(transparent)]
    Io(io::Error),

    #[error("not a directory")]
    NotDirectory,

    #[error("not a file")]
    NotFile,

    #[error("insufficient permissions")]
    InsufficientPermissions,

    #[error("too permissive")]
    FileTooPermissive,

    #[error("directory hierarchy too permissive")]
    DirectoryHierarchyTooPermissive,

    #[error("not an absolute path")]
    PathNotAbsolute,

    #[error("not found")]
    NotFound,
}
