use std::io;
use josekit::JoseError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AccessTokenGeneratorError {
    #[error("cryptographic operation failed: {0}")]
    Crypto(#[from] JoseError),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("access token serialization failed: {0}")]
    SessionIdSerialization(serde_json::Error),
}
