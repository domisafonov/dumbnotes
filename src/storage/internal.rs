use std::ffi::OsString;
use std::ops::Add;
use std::path::PathBuf;
use std::os::unix::prelude::*;
use std::str::FromStr;
use futures_util::future::join_all;
use rand::rngs::StdRng;
use time::UtcDateTime;
use tokio::io;
use tokio::io::AsyncReadExt;
use uuid::fmt::Hyphenated;
use uuid::Uuid;

use crate::config::app_config::AppConfig;
use crate::data::{Note, NoteInfo, NoteMetadata};
use crate::storage::errors::StorageError;
use crate::util::StrExt;

use io_trait::Metadata;
use io_trait::NoteStorageIo;
use io_trait::ProductionNoteStorageIo;
use crate::rng::SyncRng;
use crate::username_string::UsernameString;

mod io_trait;
#[cfg(test)] mod tests;

const REQUIRED_UNIX_PERMISSIONS: u32 = 0o700;
const HYPHENED_UUID_SIZE: usize = 36;
const TMP_FILENAME_INFIX: &str = ".tmp.";

pub type NoteStorage = NoteStorageImpl<ProductionNoteStorageIo>;

#[allow(private_bounds)]
#[derive(Debug)]
pub struct NoteStorageImpl<Io: NoteStorageIo> {
    io: Io,
    basedir: PathBuf,
    max_note_len: u64,
    max_note_name_len: u64,
}

impl NoteStorage {
    pub async fn new(
        app_config: &AppConfig,
        rng: SyncRng<StdRng>,
    ) -> Result<NoteStorage, StorageError> {
        Self::new_internal(
            app_config,
            ProductionNoteStorageIo::new(rng),
        ).await
    }
}

#[allow(private_bounds)]
impl<Io: NoteStorageIo> NoteStorageImpl<Io> {
    async fn new_internal(
        app_config: &AppConfig,
        io: Io,
    ) -> Result<NoteStorageImpl<Io>, StorageError> {
        let meta = io.metadata(&app_config.data_directory).await?;
        if !meta.is_dir {
            return Err(StorageError::DoesNotExist);
        }
        validate_note_root_permissions(&io, &meta)?;
        Ok(NoteStorageImpl {
            io,
            basedir: app_config.data_directory.clone(),
            max_note_len: app_config.max_note_size,
            max_note_name_len: app_config.max_note_name_size,
        })
    }

    pub async fn read_note(
        &mut self,
        username: &UsernameString,
        note_id: Uuid,
    ) -> Result<Note, StorageError> {
        let path = self.get_note_path(username, note_id);
        let (file, file_size) = self.io.open_file(&path).await?;
        if file_size > self.max_note_len {
            return Err(StorageError::TooBigError);
        }
        let contents = read_limited_utf8_lossy(self.max_note_len, file).await?;
        let (name, contents) = contents.split_once('\n')
            .unwrap_or((&contents, ""));
        if name.len() > self.max_note_len as usize {
            return Err(StorageError::TooBigError);
        }
        Ok(
            Note {
                id: note_id,
                name: name.nonblank_to_some(),
                contents: contents.to_owned(),
            }
        )
    }

    pub async fn write_note(
        &mut self,
        username: &UsernameString,
        note: &Note,
    ) -> Result<(), StorageError> {
        let filename = self.get_note_path(username, note.id);
        let tmp_filename = self.get_note_tmp_path(username, note.id);
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
        // TODO: reimplement with `scandir()` (needs an async implementation)
        //  and a data limit
        let mut read = self.io.read_dir(self.get_user_dir(username)).await?;
        let mut ret = Vec::new();
        while let Some(entry) = read.next_entry().await? {
            if let Some(uuid) = Self::try_extract_uuid(entry.file_name()) {
                ret.push(
                    NoteMetadata { 
                        id: uuid,
                        mtime: UtcDateTime
                            ::from_unix_timestamp(
                                entry.metadata().await
                                    .map(|nm| nm.mtime())
                                    .unwrap_or(0)
                            )
                            .unwrap_or(UtcDateTime::MIN),
                    }
                )
            }
        }
        ret.sort_by_key(|nm| nm.mtime);
        Ok(ret)
    }
    
    pub async fn get_note_details(
        &mut self,
        username: &UsernameString,
        notes: impl IntoIterator<Item=NoteMetadata>,
    ) -> Result<Vec<Option<NoteInfo>>, StorageError> {
        Ok(
            join_all(
                notes.into_iter()
                    .map(async |nm| {
                        let file = self.io
                            .open_file(self.get_note_path(username, nm.id))
                            .await
                            .map(|(file, _)| Some(file))
                            .unwrap_or_else(|e|
                                // TODO: log
                                None
                            )?;
                        let buf = read_limited_utf8_lossy(self.max_note_name_len, file)
                            .await
                            .map(Some)
                            .unwrap_or_else(|e|
                                // TODO: log
                                None
                            )?;
                        let name = buf.split_once('\n')
                            .map(|(name, _)| name)
                            .unwrap_or(&buf);
                        Some(
                            NoteInfo {
                                metadata: nm,
                                name: name.nonblank_to_some(),
                            }
                        )
                    })
            ).await
        )
    }
    
    pub async fn delete_note(
        &mut self,
        username: &UsernameString,
        id: Uuid,
    ) -> Result<(), StorageError> {
        Ok(
            self.io
                .remove_file(self.get_note_path(username, id))
                .await?
        )
    }

    fn get_user_dir(&self, username: &UsernameString) -> PathBuf {
        self.basedir.join(username as &str)
    }
    
    fn get_note_path(&self, username: &UsernameString, uuid: Uuid) -> PathBuf {
        self.get_user_dir(username).join(uuid.hyphenated().to_string())
    }

    fn get_note_tmp_path(
        &self,
        username: &UsernameString,
        uuid: Uuid,
    ) -> PathBuf {
        // TODO: maybe, guarantee atomic file creation upstream instead
        //  of using a uuid suffix
        self.get_user_dir(username)
            .join(
                uuid.hyphenated().to_string() +
                    TMP_FILENAME_INFIX
                    + &self.io.generate_uuid().hyphenated().to_string()
            )
    }

    fn try_extract_uuid(filename: OsString) -> Option<Uuid> {
        Some(filename)
            .filter(|n| n.as_bytes().len() >= HYPHENED_UUID_SIZE)
            .map(|v| String::from_utf8(v.as_bytes()[0..HYPHENED_UUID_SIZE].to_owned()))
            .transpose()
            .unwrap_or_default()
            .filter(|v| !v.chars().any(|c| c.is_uppercase()))
            .map(|v| Hyphenated::from_str(&v))
            .transpose()
            .unwrap_or_default()
            .map(Hyphenated::into_uuid)
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
) -> Result<String, io::Error> {
    let mut buf = Vec::with_capacity(limit as usize);
    io::BufReader::new(reader).take(limit).read_to_end(&mut buf).await?;
    Ok(
        String::from_utf8_lossy(&buf)
            .replace(std::char::REPLACEMENT_CHARACTER, "")
    )
}

fn format_note(note: &Note) -> String {
    String
        ::with_capacity(
        note.name.as_ref().map(String::len).unwrap_or(0) +
            "\n".len() +
            note.contents.len()
        )
        .add(note.name.as_deref().unwrap_or(""))
        .add("\n")
        .add(&note.contents)
}
