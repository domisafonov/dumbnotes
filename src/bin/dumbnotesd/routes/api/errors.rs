use crate::routes::api::protobuf::errors::MappingError;
use prost::DecodeError;
use thiserror::Error;
use dumbnotes::username_string::UsernameParseError;

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
}

impl From<UsernameParseError> for ProtobufRequestError {
    fn from(err: UsernameParseError) -> Self {
        ProtobufRequestError::SemanticDecode(err.into())
    }
}

pub(super) trait OptionExt<T> {
    fn ok_or_mapping_error(self, e: MappingError) -> Result<T, ProtobufRequestError>;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_mapping_error(self, e: MappingError) -> Result<T, ProtobufRequestError> {
        self.ok_or(ProtobufRequestError::SemanticDecode(e))
    }
}
