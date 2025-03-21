use std::borrow::Cow;
use std::io::Error as IoError;
use std::ops::Add;
use std::path::PathBuf;

use tokio::{fs, io};
use tokio::io::AsyncReadExt;
use uuid::Uuid;
use io_trait::NoteStorageIo;

use crate::config::{UsernameString, MAX_NOTE_LEN};
use crate::data::Note;
use crate::storage::errors::StorageError;

use io_trait::Metadata;
use io_trait::ProductionNoteStorageIo;

mod io_trait;
#[cfg(test)] mod tests;

const REQUIRED_UNIX_PERMISSIONS: u32 = 0o700;

pub type NoteStorage = NoteStorageImpl<ProductionNoteStorageIo>;

#[allow(private_bounds)]
pub struct NoteStorageImpl<Io: NoteStorageIo> {
    io: Io,
    basedir: PathBuf,
}

impl NoteStorage {
    pub async fn new(
        basedir: &str
    ) -> Result<NoteStorage, StorageError> {
        Self::new_internal(basedir, ProductionNoteStorageIo {}).await
    }
}

#[allow(private_bounds)]
impl<Io: NoteStorageIo> NoteStorageImpl<Io> {
    async fn new_internal(
        basedir: &str,
        mut io: Io
    ) -> Result<NoteStorageImpl<Io>, StorageError> {
        let path = PathBuf::from(basedir);
        let meta = io.metadata(&path).await?;
        if !meta.is_dir {
            return Err(StorageError::DirectoryDoesNotExist);
        }
        validate_note_root_permissions(&io, &meta)?;
        Ok(NoteStorageImpl { io, basedir: path })
    }

    pub async fn read_note(
        &mut self,
        username: &UsernameString,
        note_id: Uuid,
    ) -> Result<Note, StorageError> {
        let path = self.get_user_dir(username).join(note_id.to_string());
        let (file, file_size) = self.io.open_file(&path).await?;
        if file_size > MAX_NOTE_LEN {
            return Err(StorageError::TooBigError);
        }
        let contents = read_limited_utf8_lossy(MAX_NOTE_LEN, file).await?;
        let (name, contents) = contents.split_once('\n')
            .unwrap_or((&contents, ""));
        Ok(
            Note {
                id: note_id,
                name: Some(name).filter(|n| !n.trim().is_empty())
                    .map(str::to_owned),
                contents: contents.to_owned(),
            }
        )
    }

    pub async fn write_note(
        &mut self,
        username: &UsernameString,
        note: &Note,
    ) -> Result<(), IoError> {
        let user_dir = self.get_user_dir(username);
        let filename = user_dir.join(note.id.to_string());
        let tmp_filename = user_dir.join(Uuid::new_v4().to_string());
        self.io.write_file(&tmp_filename, format_note(note)).await?;
        if let Err(e) = self.io.rename_file(&tmp_filename, &filename).await {
            if let Err(e) = self.io.remove_file(&tmp_filename).await {
                log::error!("TODO");
            }
            return Err(e)
        }
        Ok(())
    }

    fn get_user_dir(&self, username: &UsernameString) -> PathBuf { // TODO: change to get_note_path
        self.basedir.join(username as &str)
    }
}

pub fn validate_note_root_permissions(
    io: &impl NoteStorageIo,
    meta: &Metadata,
) -> Result<(), StorageError> {
    let uid = io.getuid();
    if meta.uid != uid
        || meta.mode & REQUIRED_UNIX_PERMISSIONS != REQUIRED_UNIX_PERMISSIONS {
        return Err(StorageError::PermissionError)
    }
    Ok(())
}

async fn read_limited_utf8_lossy<R: io::AsyncRead + Unpin>(
    limit: u64,
    reader: R
) -> Result<String, StorageError> {
    let mut buf = Vec::with_capacity(limit as usize);
    io::BufReader::new(reader).take(limit).read_to_end(&mut buf).await?;
    Ok(
        match String::from_utf8_lossy(&buf) {
            Cow::Borrowed(_) => unsafe { String::from_utf8_unchecked(buf) },
            owned@Cow::Owned(_) => owned.into_owned(),
        }
    )
}

fn format_note(note: &Note) -> String {
    String::with_capacity(
        note.name.as_ref().map(String::len).unwrap_or(0) +
            "\n".len() +
            note.contents.len()
    )
        .add(note.name.as_deref().unwrap_or(""))
        .add("\n")
        .add(&note.contents)
}
