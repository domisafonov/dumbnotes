use futures::future::join_all;
use log::{debug, error, trace};
use std::ffi::OsString;
use std::io::ErrorKind;
use std::ops::Add;
use std::os::unix::prelude::*;
use std::path::PathBuf;
use std::str::FromStr;
use time::UtcDateTime;
use tokio::io;
use tokio::io::AsyncReadExt;
use uuid::fmt::Hyphenated;
use uuid::Uuid;

use crate::config::app_config::AppConfig;
use crate::data::{Note, NoteInfo, NoteMetadata};
use crate::storage::errors::StorageError;
use crate::util::{send_fut_lifetime_workaround, StrExt};

use crate::username_string::UsernameStr;
use io_trait::NoteStorageIo;
use io_trait::ProductionNoteStorageIo;
use crate::lib_constants::NOTES_DIRECTORY_PATH;
use crate::nix::CheckAccessError;
use crate::storage::internal::io_trait::OpenFile;

mod io_trait;
#[cfg(test)] mod tests;

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
    ) -> Result<NoteStorage, StorageError> {
        Self::new_internal(
            Self::get_notes_dir(app_config),
            app_config.max_note_size,
            app_config.max_note_name_size,
            ProductionNoteStorageIo::new(),
        ).await
    }

    pub fn get_notes_dir(app_config: &AppConfig) -> PathBuf {
        app_config.data_directory.join("notes")
    }
}

#[allow(private_bounds)]
impl<Io: NoteStorageIo> NoteStorageImpl<Io> {
    async fn new_internal(
        notes_dir: PathBuf,
        max_note_size: u64,
        max_note_name_size: u64,
        io: Io,
    ) -> Result<NoteStorageImpl<Io>, StorageError> {
        debug!(
            "creating note storage at {}",
            notes_dir.display(),
        );
        match io.validate_notes_path(&notes_dir) {
            Ok(_) => {},
            Err(CheckAccessError::NotFound) =>
                return Err(StorageError::DataDirNotInitialized),
            Err(e) => return Err(StorageError::CheckAccessError(e)),
        }
        Ok(NoteStorageImpl {
            io,
            basedir: notes_dir,
            max_note_len: max_note_size,
            max_note_name_len: max_note_name_size,
        })
    }

    pub async fn read_note(
        &self,
        username: &UsernameStr,
        note_id: Uuid,
    ) -> Result<Note, StorageError> {
        let path = self.get_note_path(username, note_id);
        debug!(
            "reading note {note_id} for user \"{username}\" at \"{}\"",
            path.display(),
        );
        let file = self.io
            .open_file(path)
            .await
            .map_err(|e| match e.kind() {
                ErrorKind::NotFound => StorageError::NoteNotFound,
                _ => StorageError::Io(e),
            })?;
        if file.size > self.max_note_len {
            return Err(StorageError::TooBig);
        }
        let contents = read_limited_utf8_lossy(self.max_note_len, file.file)
            .await?;
        let (name, contents) = contents.split_once('\n')
            .unwrap_or((&contents, ""));
        trace!(
            "read a note {note_id} with title \"{name}\" \
                and contents \"{contents}\""
        );
        if name.len() > self.max_note_len as usize {
            return Err(StorageError::TooBig);
        }
        Ok(
            Note {
                metadata: NoteMetadata {
                    id: note_id,
                    mtime: UtcDateTime::from_unix_timestamp(file.mtime)?,
                },
                name: name.nonblank_to_some(),
                contents: contents.to_owned(),
            }
        )
    }

    // TODO: write mtime
    pub async fn write_note(
        &self,
        username: &UsernameStr,
        note: &Note,
    ) -> Result<(), StorageError> {
        let filename = self.get_note_path(username, note.metadata.id);
        debug!(
            "writing note {} for user \"{username}\" to \"{}\"",
            note.metadata.id,
            filename.display(),
        );
        let tmp_filename = self
            .get_note_tmp_path(username, note.metadata.id);
        trace!(
            "tmp filename for note {}: \"{}\"",
            note.metadata.id,
            tmp_filename.display(),
        );
        self.io.write_file(&tmp_filename, format_note(note)).await?;
        trace!(
            "renaming tmp file \"{}\" for note \"{}\"",
            tmp_filename.display(),
            note.metadata.id,
        );
        if let Err(e) = self.io.rename_file(&tmp_filename, &filename).await {
            error!(
                "failed to rename tmp file \"{}\" for note {}: {e}",
                tmp_filename.display(),
                note.metadata.id,
            );
            if let Err(e) = self.io.remove_file(&tmp_filename).await {
                error!(
                    "failed to remove tmp file \"{}\" for note {}",
                    tmp_filename.display(),
                    e,
                );
            }
            return Err(e.into())
        }
        Ok(())
    }

    pub async fn list_notes(
        &self,
        username: &UsernameStr,
    ) -> Result<Vec<NoteMetadata>, StorageError> {
        debug!("listing notes for user \"{username}\"");
        // TODO: reimplement with `scandir()` (needs an async implementation)
        //  and a data limit
        let mut read = self.io.read_dir(self.get_user_dir(username)).await?;
        let mut ret = Vec::new();
        while let Some(entry) = read.next_entry().await? {
            trace!("read dir entry \"{entry:?}\" for user \"{username}\"");
            if let Some(uuid) = Self::try_extract_uuid(entry.file_name()) {
                trace!(
                    "dir entry \"{entry:?}\" for user \"{username}\" \
                        accepted with id {uuid}"
                );
                ret.push(
                    NoteMetadata { 
                        id: uuid,
                        mtime: UtcDateTime
                            ::from_unix_timestamp(
                                entry.metadata().await
                                    .map(|nm| nm.mtime())?
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
        &self,
        username: &UsernameStr,
        notes: impl IntoIterator<Item=NoteMetadata>,
    ) -> Result<Vec<Option<NoteInfo>>, StorageError> {
        debug!("getting note details for user \"{username}\"");
        Ok(
            send_fut_lifetime_workaround(join_all(
                notes.into_iter()
                    .map(async |nm| {
                        trace!(
                            "filling note details for note {} for user \"{username}\"",
                            nm.id,
                        );
                        let file = self.io
                            .open_file(self.get_note_path(username, nm.id))
                            .await
                            .map(|OpenFile { file, .. }| Some(file))
                            .unwrap_or_else(|e| {
                                error!(
                                    "failed to open note {} for user \"{username}\": {e}",
                                    nm.id,
                                );
                                None
                            })?;
                        trace!(
                            "open note {} for user \"{username}\", reading",
                            nm.id,
                        );
                        let buf = send_fut_lifetime_workaround(read_limited_utf8_lossy(self.max_note_name_len, file))
                            .await
                            .map(Some)
                            .unwrap_or_else(|e| {
                                error!(
                                    "failed to read note {} for user \"{username}\": {e}",
                                    nm.id,
                                );
                                None
                            })?;
                        let name = buf.split_once('\n')
                            .map(|(name, _)| name)
                            .unwrap_or(&buf);
                        trace!(
                            "parsed note title \"{name}\" of note {} for user \"{username}\"",
                            nm.id,
                        );
                        Some(
                            NoteInfo {
                                metadata: nm,
                                name: name.nonblank_to_some(),
                            }
                        )
                    })
            )).await
        )
    }
    
    pub async fn delete_note(
        &self,
        username: &UsernameStr,
        id: Uuid,
    ) -> Result<(), StorageError> {
        debug!("deleting note {id} for user \"{username}\"");
        Ok(
            self.io
                .remove_file(self.get_note_path(username, id))
                .await?
        )
    }

    fn get_user_dir(&self, username: &UsernameStr) -> PathBuf {
        self.basedir.join(NOTES_DIRECTORY_PATH).join(username as &str)
    }

    fn get_note_path(&self, username: &UsernameStr, uuid: Uuid) -> PathBuf {
        self.get_user_dir(username).join(uuid.hyphenated().to_string())
    }

    fn get_note_tmp_path(
        &self,
        username: &UsernameStr,
        uuid: Uuid,
    ) -> PathBuf {
        self.get_user_dir(username)
            .join(
                uuid.hyphenated().to_string() +
                    TMP_FILENAME_INFIX +
                    &self.io.generate_uuid().hyphenated().to_string()
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

async fn read_limited_utf8_lossy<R: io::AsyncRead + Unpin + Send>(
    limit: u64,
    reader: R,
) -> Result<String, io::Error> {
    // TODO: reimplement manually to log trimming and lossy conversions
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
