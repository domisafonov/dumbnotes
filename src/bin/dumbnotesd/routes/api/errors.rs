use std::error::Error;
use std::fmt::{Display, Formatter};
use prost::DecodeError;

#[derive(Debug)]
pub enum ProtobufRequestError {
    DecodeError(DecodeError),
    IoError(std::io::Error),
    TooLarge,
}

impl Display for ProtobufRequestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtobufRequestError::DecodeError(e) => f.write_fmt(format_args!("{}", e)),
            ProtobufRequestError::IoError(e) => f.write_fmt(format_args!("{}", e)),
            ProtobufRequestError::TooLarge => f.write_str("Request too large"),
        }
    }
}

impl Error for ProtobufRequestError {}

impl From<DecodeError> for ProtobufRequestError {
    fn from(err: DecodeError) -> Self {
        ProtobufRequestError::DecodeError(err)
    }
}

impl From<std::io::Error> for ProtobufRequestError {
    fn from(err: std::io::Error) -> Self {
        ProtobufRequestError::IoError(err)
    }
}