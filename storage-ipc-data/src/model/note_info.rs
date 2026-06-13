use data::NoteInfo;
use protobuf_common::{MappingError, OptionExt, ProtobufRequestError};

use crate::bindings;

impl From<NoteInfo> for bindings::NoteInfo {
    fn from(value: NoteInfo) -> Self {
        bindings::NoteInfo {
            metadata: Some(value.metadata.into()),
            name: value.name,
        }
    }
}

impl TryFrom<bindings::NoteInfo> for NoteInfo {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::NoteInfo) -> Result<Self, Self::Error> {
        Ok(
            NoteInfo {
                metadata: value.metadata
                    .ok_or_mapping_error(MappingError::missing("metadata"))?
                    .try_into()?,
                name: value.name,
            }
        )
    }
}
