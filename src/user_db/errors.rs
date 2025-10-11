use std::fmt;
use std::io::Error as IoError;

#[derive(Debug)]
pub enum UserDbError {
    IoError(IoError),
    ParsingError {
        message: String,
    },
}

impl fmt::Display for UserDbError { // TODO: prettier strings
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UserDbError::IoError(_) => std::fmt::Debug::fmt(&self, f),
            UserDbError::ParsingError { message } =>
                f.write_fmt(format_args!("User db parsing error: {}", message))
        }
    }
}
impl std::error::Error for UserDbError {}

impl From<IoError> for UserDbError {
    fn from(e: IoError) -> Self {
        Self::IoError(e)
    }
}

impl From<toml::de::Error> for UserDbError {
    fn from(e: toml::de::Error) -> Self {
        UserDbError::ParsingError {
            message: format!("{e}"),
        }
    }
}
