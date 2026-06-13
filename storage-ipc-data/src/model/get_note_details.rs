use std::str::FromStr;

use data::{NoteInfo, NoteMetadata, UsernameString};
use log::error;
use protobuf_common::{MappingError, OptionExt, ProtobufRequestError};
use crate::bindings::{self, NotesMetadata};
use bindings::StorageError;

#[derive(Debug)]
pub struct GetNoteDetailsRequest {
    pub username: UsernameString,
    pub notes_metadata: Vec<NoteMetadata>,
}

#[derive(Debug)]
pub enum GetNoteDetailsResponse {
    Notes(Vec<Option<NoteInfo>>),
    Error(StorageError),
}

impl TryFrom<bindings::GetNoteDetailsRequest> for GetNoteDetailsRequest {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::GetNoteDetailsRequest) -> Result<Self, ProtobufRequestError> {
        Ok(
            GetNoteDetailsRequest {
                username: UsernameString::from_str(&value.username)?,
                notes_metadata: value.notes_metadata
                    .ok_or_mapping_error(MappingError::missing("notes_metadata"))?
                    .notes_metadata
                    .into_iter()
                    .filter_map(|v| {
                        match NoteMetadata::try_from(v) {
                            Ok(note_info) => Some(note_info),
                            Err(e) => {
                                error!("failed to parse note info protobuf response: {e}");
                                None
                            }
                        }
                    })
                    .collect(),
            }
        )
    }
}

impl TryFrom<bindings::response::Response> for GetNoteDetailsResponse {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::response::Response) -> Result<Self, ProtobufRequestError> {
        use bindings:: get_note_details_response::Response;
        let value = match value {
            bindings::response::Response::GetNoteDetails(value) => value,
            _ => return Err(MappingError::UnexpectedEnumVariant.into()),
        };
        Ok(
            match value.response.ok_or_mapping_error(MappingError::missing("response"))? {
                Response::NotesInfo(notes) => GetNoteDetailsResponse::Notes(
                    notes.notes_info
                        .into_iter()
                        .map(|mn|
                            mn.note_info.map(|v|
                                NoteInfo::try_from(v)
                                    .inspect_err(|e| error!("failed to parse note protobuf response: {e}"))
                                    .ok()
                            )
                        )
                        .flatten()
                        .collect()
                ),
                Response::Error(e) => GetNoteDetailsResponse::Error(e.try_into()?),
            }
        )
    }
}

impl From<GetNoteDetailsRequest> for bindings::GetNoteDetailsRequest {
    fn from(value: GetNoteDetailsRequest) -> Self {
        bindings::GetNoteDetailsRequest {
            username: value.username.into_string(),
            notes_metadata: Some(
                NotesMetadata {
                    notes_metadata: value.notes_metadata
                        .into_iter()
                        .map(bindings::NoteMetadata::from)
                        .collect(),
                }
            ),
        }
    }
}

impl From<GetNoteDetailsResponse> for bindings::response::Response {
    fn from(value: GetNoteDetailsResponse) -> Self {
        use bindings::get_note_details_response::Response;
        bindings::response::Response::GetNoteDetails(
            bindings::GetNoteDetailsResponse {
                response: Some(
                    match value {
                        GetNoteDetailsResponse::Notes(notes) => Response::NotesInfo(
                            bindings::MaybeNotesInfo {
                                notes_info: notes
                                    .into_iter()
                                    .map(|mn| bindings::MaybeNoteInfo {
                                        note_info: mn.map(bindings::NoteInfo::from)
                                    })
                                    .collect(),
                            }
                        ),
                        GetNoteDetailsResponse::Error(e) => Response::Error(e.into()),
                    }
                ),
            }
        )
    }
}
