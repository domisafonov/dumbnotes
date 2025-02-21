use std::borrow::Cow;
use std::fs::Metadata;
use std::io::Error as IoError;
use std::ops::Add;
use std::path::{Path, PathBuf};

#[cfg(unix)] use std::os::unix::prelude::*;

use tokio::{fs, io};
use tokio::io::AsyncReadExt;
use uuid::Uuid;

use crate::config::{UsernameString, MAX_NOTE_LEN};
use crate::data::Note;
use crate::storage::errors::StorageError;

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
        if !meta.is_dir() {
            return Err(StorageError::DirectoryDoesNotExist);
        }
        validate_note_root_permissions(&meta)?;
        Ok(NoteStorageImpl { io, basedir: path })
    }

    pub async fn read_note(
        &mut self,
        username: &UsernameString,
        note_id: Uuid,
    ) -> Result<Note, StorageError> {
        let path = self.get_user_dir(username).join(note_id.to_string());
        let (file, metadata) = self.io.open_file(&path).await?;
        let file_size = metadata.len();
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

trait NoteStorageIo {
    async fn metadata(&mut self, path: &Path) -> io::Result<Metadata> {
        fs::metadata(path).await
    }

    async fn open_file(
        &mut self,
        path: &Path
    ) -> io::Result<(impl io::AsyncRead + Unpin, Metadata)> {
        let file = fs::File::open(path).await?;
        let metadata = file.metadata().await?;
        Ok((file, metadata))
    }

    async fn write_file(
        &mut self,
        path: &Path,
        data: String
    ) -> io::Result<()> {
        fs::write(path, data).await
    }

    async fn rename_file(
        &mut self,
        from: &Path,
        to: &Path,
    ) -> io::Result<()> {
        fs::rename(from, to).await
    }

    async fn remove_file(&mut self, path: &Path) -> io::Result<()> {
        fs::remove_file(path).await
    }
}
pub struct ProductionNoteStorageIo {}
impl NoteStorageIo for ProductionNoteStorageIo {}

#[cfg(unix)]
fn validate_note_root_permissions(
    meta: &Metadata
) -> Result<(), StorageError> {
    let uid = unsafe { libc::getuid() };
    if meta.uid() != uid
        || meta.mode() & REQUIRED_UNIX_PERMISSIONS != REQUIRED_UNIX_PERMISSIONS {
        return Err(StorageError::PermissionError)
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_note_root_permissions(
    meta: &Metadata
) -> Result<(), StorageError> {
    todo!()
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
