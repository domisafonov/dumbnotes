use std::io;
use josekit::JoseError;
use thiserror::Error;
use crate::username_string::UsernameParseError;

#[derive(Debug, Error)]
pub enum AccessTokenDecoderError {
    #[error("cryptographic operation failed: {0}")]
    Crypto(#[from] JoseError),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("invalid access token payload: {0}")]
    PayloadParse(serde_json::Error),

    #[error("invalid username: {0}")]
    PayloadUsername(#[from] UsernameParseError),

    #[error("missing {part} in the payload")]
    PayloadMissing {
        part: &'static str,
    },
}
