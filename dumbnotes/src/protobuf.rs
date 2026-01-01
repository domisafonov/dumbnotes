use thiserror::Error;
use prost::DecodeError;
use crate::username_string::UsernameParseError;

#[derive(Debug, Error)]
pub enum MappingError {
    #[error("missing field: {name}")]
    MissingField {
        name: &'static str,
    },

    #[error("invalid username: {0}")]
    UsernameParse(#[from] UsernameParseError),
    
    #[error("unexpected enum variant")]
    UnexpectedEnumVariant,
}

impl MappingError {
    pub fn missing(name: &'static str) -> Self {
        MappingError::MissingField { name }
    }
}

#[derive(Debug, Error)]
pub enum ProtobufRequestError {
    #[error("invalid protobuf message: {0}")]
    ProtobufDecode(#[from] DecodeError),

    #[error("invalid protobuf message semantics: {0}")]
    SemanticDecode(#[from] MappingError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("request too large")]
    RequestTooLarge,

    #[error("incorrect uuid")]
    IncorrectUuid(#[from] uuid::Error),

    #[error("incorrect timestamp")]
    IncorrectTimestamp(#[from] time::error::ComponentRange),

    #[error("invalid enum value: {0}")]
    UnknownEnum(#[from] prost::UnknownEnumValue),
}

impl From<UsernameParseError> for ProtobufRequestError {
    fn from(err: UsernameParseError) -> Self {
        ProtobufRequestError::SemanticDecode(err.into())
    }
}

pub trait OptionExt<T> {
    fn ok_or_mapping_error(self, e: MappingError) -> Result<T, ProtobufRequestError>;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_mapping_error(self, e: MappingError) -> Result<T, ProtobufRequestError> {
        self.ok_or(ProtobufRequestError::SemanticDecode(e))
    }
}
