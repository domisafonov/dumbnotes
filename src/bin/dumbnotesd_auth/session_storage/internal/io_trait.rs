use async_trait::async_trait;
use dumbnotes::rng::make_uuid;
use rand::RngCore;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use boolean_enums::gen_boolean_enum;
use log::{debug, trace};
use time::OffsetDateTime;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;
use uuid::Uuid;
use crate::app_constants::{REFRESH_TOKEN_SIZE, SESSION_STORAGE_READ_BUF_SIZE};
use crate::session_storage::internal::data::SessionsData;
use crate::session_storage::SessionStorageError;

#[async_trait]
pub trait SessionStorageIo: Send + Sync + 'static {
    async fn read_session_file(
        &self,
    ) -> Result<SessionsData, SessionStorageError>;

    async fn write_session_file(
        &self,
        sessions_data: &SessionsData,
    ) -> Result<(), SessionStorageError>;

    fn gen_refresh_token(&self) -> Vec<u8>;

    fn get_time(&self) -> OffsetDateTime;
    
    fn generate_uuid(&self) -> Uuid;
}

pub struct ProductionSessionStorageIo {
    db_file: Mutex<FileWithIno>, // holds a file lock
    session_file_path: PathBuf,
}

struct FileWithIno {
    file: File,
    ino: u64,
}

impl ProductionSessionStorageIo {
    pub async fn new(
        session_file_path: impl AsRef<Path> + Send,
    ) -> Result<Self, SessionStorageError> {
        trace!(
            "creating session storage at {}",
            session_file_path.as_ref().display(),
        );
        let path = PathBuf::from(session_file_path.as_ref());
        let file = Self::open_file(
            &path,
            CreateIfDoesNotExist::Yes,
        )?;
        debug!("opened the session db file at {}", path.display());
        file.try_lock().map_err(SessionStorageError::LockingFailed)?;
        debug!("locked the session db file at {}", path.display());
        let file = File::from_std(file);
        let ino = file.metadata().await?.ino();
        trace!("session db file has ino: {ino}");
        Ok(
            ProductionSessionStorageIo {
                db_file: Mutex::new(
                    FileWithIno {
                        file,
                        ino,
                    }
                ),
                session_file_path: path,
            }
        )
    }

    fn open_file(
        session_file_path: &Path,
        create_if_does_not_exist: CreateIfDoesNotExist,
    ) -> Result<std::fs::File, SessionStorageError> {
        Ok(
            std::fs::File::options()
                .create(create_if_does_not_exist.into())
                .read(true)
                .write(true)
                .truncate(false)
                .open(session_file_path)?
        )
    }
}

#[async_trait]
impl SessionStorageIo for ProductionSessionStorageIo {
    async fn read_session_file(
        &self,
    ) -> Result<SessionsData, SessionStorageError> {
        trace!(
            "reading session storage file at \"{}\"",
            self.session_file_path.display(),
        );
        let db_file_ino = tokio::fs::metadata(&self.session_file_path).await?.ino();
        trace!("the file on session db path has ino {db_file_ino}");
        let db_file = Self::open_file(
            &self.session_file_path,
            CreateIfDoesNotExist::No,
        )?;
        let mut stored_file = self.db_file.lock().await;
        if db_file_ino != stored_file.ino {
            debug!(
                "session file at \"{}\" replaced, reapplying the lock",
                self.session_file_path.display(),
            );
            db_file.try_lock().map_err(SessionStorageError::LockingFailed)?;
            trace!(
                "locked the session db file at \"{}\"",
                self.session_file_path.display(),
            );
            *stored_file = FileWithIno {
                file: File::from_std(db_file),
                ino: db_file_ino,
            }
        };
        stored_file.file.rewind().await?;
        let mut read_buf = String::with_capacity(SESSION_STORAGE_READ_BUF_SIZE);
        stored_file.file.read_to_string(&mut read_buf).await?;
        trace!(
            "read the session db file at {}: \"{read_buf}\"",
            self.session_file_path.display(),
        );
        let sessions = toml::de::from_str(&read_buf)?;
        trace!(
            "parsed the session db file at {}: \"{sessions:?}\"",
            self.session_file_path.display(),
        );
        Ok(sessions)
    }

    async fn write_session_file(
        &self,
        sessions_data: &SessionsData,
    ) -> Result<(), SessionStorageError> {
        trace!(
            "writing session storage file at \"{}\"",
            self.session_file_path.display(),
        );
        let mut db_file = self.db_file.lock().await;
        db_file.file.set_len(0).await?;
        db_file.file.rewind().await?;
        let serialized = toml::to_string(&sessions_data)?;
        trace!("sessions serialized as \"{serialized}\"");
        db_file.file.write_all(
            serialized.as_bytes(),
        ).await?;
        db_file.file.flush().await?;
        debug!(
            "finished writing session storage data at \"{}\"",
            self.session_file_path.display(),
        );
        Ok(())
    }

    fn gen_refresh_token(&self) -> Vec<u8> {
        let mut token = vec![0; REFRESH_TOKEN_SIZE];
        rand::rng().fill_bytes(token.as_mut_slice());
        token
    }

    fn get_time(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
    
    fn generate_uuid(&self) -> Uuid {
        make_uuid(&mut rand::rng())
    }
}

gen_boolean_enum!(CreateIfDoesNotExist);
