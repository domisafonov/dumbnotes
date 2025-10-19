use std::fmt::{Display, Formatter};
use std::io;
use josekit::JoseError;
use dumbnotes::username_string::UsernameParseError;

#[derive(Debug)]
pub enum AccessTokenDecoderError {
    CryptoError(JoseError),
    IoError(io::Error),
    PayloadError,
}

impl Display for AccessTokenDecoderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessTokenDecoderError::CryptoError(e) =>
                f.write_fmt(format_args!("{e}")),
            AccessTokenDecoderError::IoError(e) =>
                f.write_fmt(format_args!("{e}")),
            AccessTokenDecoderError::PayloadError =>
                f.write_str("Error decoding payload"),
        }
    }
}

impl std::error::Error for AccessTokenDecoderError {}

impl From<JoseError> for AccessTokenDecoderError {
    fn from(e: JoseError) -> AccessTokenDecoderError {
        AccessTokenDecoderError::CryptoError(e)
    }
}

impl From<io::Error> for AccessTokenDecoderError {
    fn from(e: io::Error) -> AccessTokenDecoderError {
        AccessTokenDecoderError::IoError(e)
    }
}

impl From<serde_json::error::Error> for AccessTokenDecoderError {
    fn from(_: serde_json::error::Error) -> AccessTokenDecoderError {
        AccessTokenDecoderError::PayloadError
    }
}

impl From<UsernameParseError> for AccessTokenDecoderError {
    fn from(_: UsernameParseError) -> AccessTokenDecoderError {
        AccessTokenDecoderError::PayloadError
    }
}
