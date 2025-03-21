use std::borrow::Cow;
use std::cmp::min;
use std::ops::Add;
use std::path::PathBuf;
use std::os::unix::prelude::*;
use std::str::FromStr;
use time::UtcDateTime;
use tokio::{fs, io};
use tokio::io::AsyncReadExt;
use uuid::fmt::Hyphenated;
use uuid::Uuid;
use io_trait::NoteStorageIo;

use crate::config::{UsernameString, MAX_NOTE_LEN};
use crate::data::{Note, NoteInfo, NoteMetadata};
use crate::storage::errors::StorageError;

use io_trait::Metadata;
use io_trait::ProductionNoteStorageIo;

mod io_trait;
#[cfg(test)] mod tests;

const REQUIRED_UNIX_PERMISSIONS: u32 = 0o700;
const HYPNENED_UUID_SIZE: usize = 36;

pub type NoteStorage = NoteStorageImpl<ProductionNoteStorageIo>;

#[allow(private_bounds)]
#[derive(Debug)]
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
    ) -> Result<(), StorageError> {
        let user_dir = self.get_user_dir(username);
        let filename = user_dir.join(note.id.to_string());
        let tmp_filename = user_dir.join(Uuid::new_v4().to_string());
        self.io.write_file(&tmp_filename, format_note(note)).await?;
        if let Err(e) = self.io.rename_file(&tmp_filename, &filename).await {
            if let Err(e) = self.io.remove_file(&tmp_filename).await {
                log::error!("TODO");
            }
            return Err(e.into())
        }
        Ok(())
    }
    
    pub async fn list_notes(
        &mut self,
        username: &UsernameString,
    ) -> Result<Vec<NoteMetadata>, StorageError> {
        let mut read = self.io.read_dir(self.get_user_dir(username)).await?;
        let mut ret = Vec::new();
        while let Some(entry) = read.next_entry().await? {
            // TODO: that's a mess
            let name = entry.file_name();
            if name.as_bytes().len() < HYPNENED_UUID_SIZE {
                continue
            }
            let name = String::from_utf8(name.as_bytes()[0..HYPNENED_UUID_SIZE].to_owned());
            if name.is_err() {
                continue
            }
            let name = name.unwrap();
            if name.chars().any(|c| c.is_uppercase()) {
                continue
            }
            if let Ok(uuid) = Hyphenated::from_str(&name) {
                ret.push(
                    NoteMetadata { 
                        id: uuid.into_uuid(),
                        mtime: UtcDateTime::from_unix_timestamp(entry.metadata().await?.mtime())?, // TODO: more sane handling
                    }
                )
            }
        }
        Ok(ret)
    }
    
    pub async fn get_note_details(
        &mut self,
        username: &UsernameString,
        metadata: NoteMetadata,
    ) -> Result<NoteInfo, StorageError> {
        todo!()
    }
    
    pub async fn delete_note(
        &mut self,
        username: &UsernameString,
        id: Uuid,
    ) -> Result<(), StorageError> {
        Ok(self.io.remove_file(self.get_user_dir(username).join(id.to_string())).await?)
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
