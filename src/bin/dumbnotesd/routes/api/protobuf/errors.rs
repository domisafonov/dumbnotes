use std::fmt::{Display, Formatter};
use dumbnotes::username_string::UsernameParseError;
use crate::routes::api::errors::ProtobufRequestError;

#[derive(Debug)]
pub struct MappingError;

impl Display for MappingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Error mapping data")
    }
}
impl std::error::Error for MappingError {}

impl From<UsernameParseError> for MappingError {
    fn from(_: UsernameParseError) -> Self {
        MappingError
    }
}
