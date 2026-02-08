use data::Note;
use time::UtcDateTime;
use crate::{protobuf_request, protobuf_response};
use protobuf_common::{MappingError, OptionExt, ProtobufRequestError};
use crate::model::{NoteResponse, NoteWriteRequest};
use crate::bindings;

impl TryFrom<bindings::NoteResponse> for NoteResponse {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::NoteResponse) -> Result<Self, Self::Error> {
        let info = value.info
            .ok_or_mapping_error(MappingError::missing("info"))?;
        let metadata = info.metadata
            .ok_or_mapping_error(MappingError::missing("metadata"))?
            .try_into()?;
        Ok(
            NoteResponse(
                Note {
                    metadata,
                    name: info.name,
                    contents: value.contents,
                }
            )
        )
    }
}

impl From<NoteResponse> for bindings::NoteResponse {
    fn from(value: NoteResponse) -> Self {
        bindings::NoteResponse {
            info: Some(
                bindings::NoteInfo {
                    metadata: Some(
                        value.0.metadata.into(),
                    ),
                    name: value.0.name,
                },
            ),
            contents: value.0.contents,
        }
    }
}

impl TryFrom<bindings::NoteWriteRequest> for NoteWriteRequest {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::NoteWriteRequest) -> Result<Self, Self::Error> {
        Ok(
            NoteWriteRequest {
                mtime: UtcDateTime::from_unix_timestamp(value.mtime)?,
                name: value.name,
                contents: value.contents,
            }
        )
    }
}

impl From<NoteWriteRequest> for bindings::NoteWriteRequest {
    fn from(value: NoteWriteRequest) -> Self {
        bindings::NoteWriteRequest {
            mtime: value.mtime.unix_timestamp(),
            name: value.name,
            contents: value.contents,
        }
    }
}

protobuf_request!(bindings::NoteWriteRequest, NoteWriteRequest);
protobuf_response!(bindings::NoteResponse, NoteResponse);
