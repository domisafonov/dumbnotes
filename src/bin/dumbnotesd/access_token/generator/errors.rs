use std::fmt::{Display, Formatter};
use std::io;
use josekit::JoseError;

#[derive(Debug)]
pub enum AccessTokenGeneratorError {
    CryptoError(JoseError),
    IoError(io::Error),
    SerializationError,
}

impl Display for AccessTokenGeneratorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessTokenGeneratorError::CryptoError(e) =>
                f.write_fmt(format_args!("{e}")),
            AccessTokenGeneratorError::IoError(e) =>
                f.write_fmt(format_args!("{e}")),
            AccessTokenGeneratorError::SerializationError =>
                f.write_str("Error serializing JWT claims"),
        }
    }
}

impl std::error::Error for AccessTokenGeneratorError {}

impl From<JoseError> for AccessTokenGeneratorError {
    fn from(e: JoseError) -> AccessTokenGeneratorError {
        AccessTokenGeneratorError::CryptoError(e)
    }
}

impl From<io::Error> for AccessTokenGeneratorError {
    fn from(e: io::Error) -> AccessTokenGeneratorError {
        AccessTokenGeneratorError::IoError(e)
    }
}

impl From<serde_json::Error> for AccessTokenGeneratorError {
    fn from(_: serde_json::Error) -> AccessTokenGeneratorError {
        AccessTokenGeneratorError::SerializationError
    }
}