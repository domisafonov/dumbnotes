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

#[derive(Clone, Debug)]
pub struct Note { // always prevalidated for Unicode and MAX_NOTE_LEN
    pub id: Uuid,
    pub name: Option<String>,
    pub contents: String,
}
