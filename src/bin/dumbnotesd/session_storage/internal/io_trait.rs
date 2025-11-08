use crate::app_constants::{REFRESH_TOKEN_SIZE, SESSION_STORAGE_READ_BUF_SIZE};
use crate::session_storage::internal::data::SessionsData;
use crate::session_storage::SessionStorageError;
use async_trait::async_trait;
use dumbnotes::rng::make_uuid;
use rand::RngCore;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;
use uuid::Uuid;

#[async_trait]
pub(super) trait SessionStorageIo: Send + Sync + 'static {
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
        let path = PathBuf::from(session_file_path.as_ref());
        let file = Self::open_file(&path)?;
        file.try_lock().map_err(SessionStorageError::LockingFailed)?;
        let file = File::from_std(file);
        let ino = file.metadata().await?.ino();
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

    fn open_file(session_file_path: &Path) -> Result<std::fs::File, SessionStorageError> {
        Ok(
            std::fs::File::options()
                .create(true)
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
        let db_file_ino = tokio::fs::metadata(&self.session_file_path).await?.ino();
        let db_file = Self::open_file(&self.session_file_path)?;
        let mut stored_file = self.db_file.lock().await;
        if db_file_ino != stored_file.ino {
            db_file.try_lock().map_err(SessionStorageError::LockingFailed)?;
            *stored_file = FileWithIno {
                file: File::from_std(db_file),
                ino: db_file_ino,
            }
        };
        stored_file.file.rewind().await?;
        let mut read_buf = String::with_capacity(SESSION_STORAGE_READ_BUF_SIZE);
        stored_file.file.read_to_string(&mut read_buf).await?;
        Ok(toml::de::from_str(&read_buf)?)
    }

    async fn write_session_file(
        &self,
        sessions_data: &SessionsData,
    ) -> Result<(), SessionStorageError> {
        let mut db_file = self.db_file.lock().await;
        db_file.file.set_len(0).await?;
        db_file.file.rewind().await?;
        db_file.file.write_all(
            toml::to_string(&sessions_data)?.as_bytes(),
        ).await?;
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
