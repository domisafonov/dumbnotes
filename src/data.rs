use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Note { // always prevalidated for Unicode and MAX_NOTE_LEN
    pub id: Uuid,
    pub name: Option<String>,
    pub contents: String,
}
