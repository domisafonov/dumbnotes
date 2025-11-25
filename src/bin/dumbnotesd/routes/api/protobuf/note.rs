use crate::protobuf_response;
use crate::routes::api::model::NoteResponse;
use crate::routes::api::protobuf::bindings;

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

protobuf_response!(bindings::NoteResponse, NoteResponse);
