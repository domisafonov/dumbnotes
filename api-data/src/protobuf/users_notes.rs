use data::{NoteInfo};
use protobuf_common::{MappingError, OptionExt, ProtobufRequestError};

use crate::protobuf_response;
use crate::model::NoteListResponse;
use crate::bindings;

impl From<NoteListResponse> for bindings::NoteListResponse {
    fn from(value: NoteListResponse) -> Self {
        bindings::NoteListResponse {
            notes_info: value.notes_info
                .into_iter()
                .map(|info| {
                    bindings::NoteInfo {
                        metadata: Some(
                            bindings::NoteMetadata {
                                id: info.metadata.id.into_bytes().to_vec(),
                                mtime: info.metadata.mtime.unix_timestamp(),
                            }
                        ),
                        name: info.name,
                    }
                })
                .collect()
        }
    }
}

impl TryFrom<bindings::NoteListResponse> for NoteListResponse {
    type Error = ProtobufRequestError;

    fn try_from(
        value: bindings::NoteListResponse,
    ) -> Result<Self, Self::Error> {
        Ok(
            NoteListResponse {
                notes_info: value.notes_info
                    .into_iter()
                    .map(|ni| -> Result<_, ProtobufRequestError> {
                        Ok(
                            NoteInfo {
                                metadata: ni.metadata
                                    .ok_or_mapping_error(MappingError::missing("metadata"))
                                    .and_then(|v| v.try_into())?,
                                name: ni.name,
                            }
                        )
                    })
                    .collect::<Result<_, _>>()?
            }
        )
    }
}

protobuf_response!(bindings::NoteListResponse, NoteListResponse);
