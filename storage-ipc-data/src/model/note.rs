use data::{Note, NoteInfo};
use protobuf_common::{MappingError, OptionExt, ProtobufRequestError};

use crate::bindings;

impl From<Note> for bindings::Note {
    fn from(value: Note) -> Self {
        bindings::Note {
            info: Some(
                bindings::NoteInfo {
                    metadata: Some(value.metadata.into()),
                    name: value.name,
                }
            ),
            contents: value.contents,
        }
    }
}

impl TryFrom<bindings::Note> for Note {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::Note) -> Result<Self, Self::Error> {
        let note_info: NoteInfo = value.info
            .ok_or_mapping_error(MappingError::missing("info"))?
            .try_into()?;
        Ok(
            Note {
                metadata: note_info.metadata,
                name: note_info.name,
                contents: value.contents,
            }
        )
    }
}
