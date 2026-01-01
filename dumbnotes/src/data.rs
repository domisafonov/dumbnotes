use time::UtcDateTime;
use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
pub struct NoteMetadata {
    pub id: Uuid,
    pub mtime: UtcDateTime,
}

#[derive(Clone, Debug)]
pub struct NoteInfo {
    pub metadata: NoteMetadata,
    pub name: Option<String>,
}

// TODO: data is always validated for MAX_NOTE_LEN
#[derive(Clone, Debug)]
pub struct Note { 
    pub metadata: NoteMetadata,
    pub name: Option<String>,
    pub contents: String,
}
