use std::str::FromStr;

use data::{Note, UsernameString};
use protobuf_common::{MappingError, OptionExt, ProtobufRequestError};
use uuid::Uuid;
use crate::bindings;

#[derive(Debug)]
pub struct ReadNoteRequest {
    pub username: UsernameString,
    pub note_id: Uuid,
}

#[derive(Debug)]
pub struct ReadNoteResponse(
    pub Result<Note, bindings::StorageError>
);

impl TryFrom<bindings::ReadNoteRequest> for ReadNoteRequest {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::ReadNoteRequest) -> Result<Self, Self::Error> {
        Ok(
            ReadNoteRequest {
                username: UsernameString::from_str(&value.username)?,
                note_id: Uuid::from_slice(&value.note_id)?,
            }
        )
    }
}

impl TryFrom<bindings::response::Response> for ReadNoteResponse {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::response::Response) -> Result<Self, Self::Error> {
        use bindings::read_note_response::Response;
        let value = match value {
            bindings::response::Response::ReadNote(value) => value,
            _ => return Err(MappingError::UnexpectedEnumVariant.into()),
        };
        Ok(
            ReadNoteResponse(
                match value.response.ok_or_mapping_error(MappingError::missing("response"))? {
                    Response::Note(note) => Ok(note.try_into()?),
                    Response::Error(e) => Err(e.try_into()?),
                }
            )
        )
    }
}

impl From<ReadNoteRequest> for bindings::ReadNoteRequest {
    fn from(value: ReadNoteRequest) -> Self {
        bindings::ReadNoteRequest {
            username: value.username.into_string(),
            note_id: value.note_id.into_bytes().to_vec(),
        }
    }
}

impl From<ReadNoteResponse> for bindings::response::Response {
    fn from(value: ReadNoteResponse) -> Self {
        bindings::response::Response::ReadNote(
            bindings::ReadNoteResponse {
                response: Some(
                    match value.0 {
                        Ok(response) => bindings::read_note_response::Response::Note(response.into()),
                        Err(e) => bindings::read_note_response::Response::Error(e.into()),
                    }
                ),
            }
        )
    }
}
