use thiserror::Error;
use dumbnotes::username_string::UsernameParseError;

#[derive(Debug, Error)]
pub enum MappingError {
    #[error("missing field: {name}")]
    MissingField {
        name: &'static str,
    },
    
    #[error("invalid username: {0}")]
    UsernameParse(#[from] UsernameParseError),
}

impl MappingError {
    pub fn missing(name: &'static str) -> Self {
        MappingError::MissingField { name }
    }
}