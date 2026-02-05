use time::UtcDateTime;
use crate::{protobuf_request, protobuf_response};
use protobuf_common::ProtobufRequestError;
use crate::model::{NoteResponse, NoteWriteRequest};
use crate::bindings;

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

protobuf_request!(bindings::NoteWriteRequest, NoteWriteRequest);
protobuf_response!(bindings::NoteResponse, NoteResponse);
