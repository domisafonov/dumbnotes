use std::io::Error as IoError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UserDbError {
    #[error(transparent)]
    Io(#[from] IoError),

    #[error("invalid user db file contents: $0")]
    Parsing(#[from] toml::de::Error),
}
