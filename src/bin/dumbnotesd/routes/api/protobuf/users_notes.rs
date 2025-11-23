use crate::protobuf_response;
use crate::routes::api::model::NoteListResponse;
use crate::routes::api::protobuf::bindings;

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

protobuf_response!(bindings::NoteListResponse, NoteListResponse);
