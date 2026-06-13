use std::str::FromStr;

use data::UsernameString;
use protobuf_common::{MappingError, ProtobufRequestError};
use uuid::Uuid;
use crate::bindings;
use bindings::StorageError;

#[derive(Debug)]
pub struct DeleteNoteRequest {
    pub username: UsernameString,
    pub note_id: Uuid,
}

#[derive(Debug)]
pub struct DeleteNoteResponse(pub Option<StorageError>);

impl TryFrom<bindings::DeleteNoteRequest> for DeleteNoteRequest {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::DeleteNoteRequest) -> Result<Self, Self::Error> {
        Ok(
            DeleteNoteRequest {
                username: UsernameString::from_str(&value.username)?,
                note_id: Uuid::from_slice(&value.note_id)?,
            }
        )
    }
}

impl TryFrom<bindings::response::Response> for DeleteNoteResponse {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::response::Response) -> Result<Self, Self::Error> {
        let value = match value {
            bindings::response::Response::DeleteNote(value) => value,
            _ => return Err(MappingError::UnexpectedEnumVariant.into()),
        };
        Ok(
            DeleteNoteResponse(
                value.error.map(|e| e.try_into()).transpose()?,
            )
        )
    }
}

impl From<DeleteNoteRequest> for bindings::DeleteNoteRequest {
    fn from(value: DeleteNoteRequest) -> Self {
        bindings::DeleteNoteRequest {
            username: value.username.into_string(),
            note_id: value.note_id.into_bytes().to_vec(),
        }
    }
}

impl From<DeleteNoteResponse> for bindings::response::Response {
    fn from(value: DeleteNoteResponse) -> Self {
        bindings::response::Response::DeleteNote(
            bindings::DeleteNoteResponse {
                error: value.0.map(StorageError::into)
            }
        )
    }
}
