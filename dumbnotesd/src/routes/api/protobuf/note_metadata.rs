use dumbnotes::data::NoteMetadata;
use crate::routes::api::protobuf::bindings;

impl From<NoteMetadata> for bindings::NoteMetadata {
    fn from(value: NoteMetadata) -> Self {
        bindings::NoteMetadata {
            id: value.id.into_bytes().to_vec(),
            mtime: value.mtime.unix_timestamp(),
        }
    }
}