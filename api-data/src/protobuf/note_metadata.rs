use data::NoteMetadata;
use protobuf_common::ProtobufRequestError;
use time::UtcDateTime;
use uuid::Uuid;
use crate::bindings;

impl From<NoteMetadata> for bindings::NoteMetadata {
    fn from(value: NoteMetadata) -> Self {
        bindings::NoteMetadata {
            id: value.id.into_bytes().to_vec(),
            mtime: value.mtime.unix_timestamp(),
        }
    }
}

impl TryFrom<bindings::NoteMetadata> for NoteMetadata {
    type Error = ProtobufRequestError;

    fn try_from(value: bindings::NoteMetadata) -> Result<Self, Self::Error> {
        Ok(
            NoteMetadata {
                id: Uuid::from_slice(&value.id)?,
                mtime: UtcDateTime::from_unix_timestamp(value.mtime)?,
            }
        )
    }
}
