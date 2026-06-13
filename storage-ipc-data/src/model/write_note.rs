use std::str::FromStr;

use data::{Note, UsernameString};
use protobuf_common::{MappingError, OptionExt, ProtobufRequestError};

use crate::bindings::{self, StorageError};

#[derive(Debug)]
pub struct WriteNoteRequest {
    pub username: UsernameString,
    pub note: Note,
}

#[derive(Debug)]
pub struct WriteNoteResponse(pub Option<StorageError>);

impl TryFrom<bindings::WriteNoteRequest> for WriteNoteRequest {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::WriteNoteRequest) -> Result<Self, Self::Error> {
        Ok(
            WriteNoteRequest {
                username: UsernameString::from_str(&value.username)?,
                note: value.note
                    .ok_or_mapping_error(MappingError::missing("note"))?
                    .try_into()?,
            }
        )
    }
}

impl TryFrom<bindings::response::Response> for WriteNoteResponse {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::response::Response) -> Result<Self, Self::Error> {
        let value = match value {
            bindings::response::Response::WriteNote(value) => value,
            _ => return Err(MappingError::UnexpectedEnumVariant.into()),
        };
        Ok(
            WriteNoteResponse(
                value.error.map(|e| e.try_into()).transpose()?,
            )
        )
    }
}

impl From<WriteNoteRequest> for bindings::WriteNoteRequest {
    fn from(value: WriteNoteRequest) -> Self {
        bindings::WriteNoteRequest {
            username: value.username.into_string(),
            note: Some(value.note.into()),
        }
    }
}

impl From<WriteNoteResponse> for bindings::response::Response {
    fn from(value: WriteNoteResponse) -> Self {
        bindings::response::Response::WriteNote(
            bindings::WriteNoteResponse {
                error: value.0.map(StorageError::into),
            }
        )
    }
}
