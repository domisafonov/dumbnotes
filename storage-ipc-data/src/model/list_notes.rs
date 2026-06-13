use std::str::FromStr;

use data::{NoteMetadata, UsernameString};
use log::error;
use protobuf_common::{MappingError, OptionExt, ProtobufRequestError};
use crate::bindings;
use bindings::StorageError;

#[derive(Debug)]
pub struct ListNotesRequest {
    pub username: UsernameString,
}

#[derive(Debug)]
pub enum ListNotesResponse {
    Notes(Vec<NoteMetadata>),
    Error(StorageError),
}

impl TryFrom<bindings::ListNotesRequest> for ListNotesRequest {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::ListNotesRequest) -> Result<Self, Self::Error> {
        Ok(
            ListNotesRequest {
                username: UsernameString::from_str(&value.username)?,
            }
        )
    }
}

impl TryFrom<bindings::response::Response> for ListNotesResponse {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::response::Response) -> Result<Self, ProtobufRequestError> {
        use bindings::list_notes_response::Response;
        let value = match value {
            bindings::response::Response::ListNotes(value) => value,
            _ => return Err(MappingError::UnexpectedEnumVariant.into()),
        };
        Ok(
            match value.response.ok_or_mapping_error(MappingError::missing("response"))? {
                Response::NotesInfo(notes_metadata) => ListNotesResponse::Notes(
                    notes_metadata.notes_metadata
                        .into_iter()
                        .filter_map(|v| {
                            match NoteMetadata::try_from(v) {
                                Ok(note_metadata) => Some(note_metadata),
                                Err(e) => {
                                    error!("failed to parse note metadata protobuf repsonse: {e}");
                                    None
                                }
                            }
                        })
                        .collect()
                ),
                Response::Error(e) => ListNotesResponse::Error(e.try_into()?),
            }
        )
    }
}

impl From<ListNotesRequest> for bindings::ListNotesRequest {
    fn from(value: ListNotesRequest) -> Self {
        bindings::ListNotesRequest {
            username: value.username.into_string(),
        }
    }
}

impl From<ListNotesResponse> for bindings::response::Response {
    fn from(value: ListNotesResponse) -> Self {
        use bindings::list_notes_response::Response;
        bindings::response::Response::ListNotes(
            bindings::ListNotesResponse {
                response: Some(
                    match value {
                        ListNotesResponse::Notes(notes_info) => Response::NotesInfo(
                            bindings::NotesMetadata {
                                notes_metadata: notes_info
                                    .into_iter()
                                    .map(bindings::NoteMetadata::from)
                                    .collect(),
                            }
                        ),
                        ListNotesResponse::Error(e) => Response::Error(e.into()),
                    }
                )
            }
        )
    }
}
