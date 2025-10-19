use crate::routes::api::protobuf::errors::MappingError;
use prost::DecodeError;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum ProtobufRequestError {
    ProtobufDecodeError(DecodeError),
    SemanticDecodeError(MappingError),
    IoError(std::io::Error),
    TooLarge,
}

impl Display for ProtobufRequestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtobufRequestError::ProtobufDecodeError(e) => f.write_fmt(format_args!("{e}")),
            ProtobufRequestError::SemanticDecodeError(e) => f.write_fmt(format_args!("{e}")),
            ProtobufRequestError::IoError(e) => f.write_fmt(format_args!("{e}")),
            ProtobufRequestError::TooLarge => f.write_str("Request too large"),
        }
    }
}

impl Error for ProtobufRequestError {}

impl From<DecodeError> for ProtobufRequestError {
    fn from(err: DecodeError) -> Self {
        ProtobufRequestError::ProtobufDecodeError(err)
    }
}

impl From<std::io::Error> for ProtobufRequestError {
    fn from(err: std::io::Error) -> Self {
        ProtobufRequestError::IoError(err)
    }
}

impl<T: Into<MappingError>> From<T> for ProtobufRequestError {
    fn from(err: T) -> Self {
        ProtobufRequestError::SemanticDecodeError(err.into())
    }
}

pub(super) trait OptionExt<T> {
    fn ok_or_mapping_error(self, e: MappingError) -> Result<T, ProtobufRequestError>;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_mapping_error(self, e: MappingError) -> Result<T, ProtobufRequestError> {
        self.ok_or(ProtobufRequestError::SemanticDecodeError(e))
    }
}
