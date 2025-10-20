use crate::app_constants::{REFRESH_TOKEN_SIZE, SESSION_STORAGE_READ_BUF_SIZE};
use crate::session_storage::internal::data::SessionsData;
use crate::session_storage::SessionStorageError;
use async_trait::async_trait;
use dumbnotes::rng::make_uuid;
use rand::RngCore;
use std::path::Path;
use time::OffsetDateTime;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;
use uuid::Uuid;

#[async_trait]
pub(super) trait SessionStorageIo: Send + Sync {
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
    db_file: Mutex<File>, // holds a file lock
}

impl ProductionSessionStorageIo {
    pub async fn new(
        session_file_path: impl AsRef<Path> + Send,
    ) -> Result<Self, SessionStorageError> {
        let std_file = std::fs::File::options()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(session_file_path)?;
        std_file.lock().map_err(SessionStorageError::LockingFailed)?;
        Ok(
            ProductionSessionStorageIo {
                db_file: Mutex::new(File::from_std(std_file)),
            }
        )
    }
}

#[async_trait]
impl SessionStorageIo for ProductionSessionStorageIo {
    async fn read_session_file(
        &self,
    ) -> Result<SessionsData, SessionStorageError> {
        let mut db_file = self.db_file.lock().await;
        db_file.rewind().await?;
        let mut read_buf = String::with_capacity(SESSION_STORAGE_READ_BUF_SIZE);
        db_file.read_to_string(&mut read_buf).await?;
        Ok(toml::de::from_str(&read_buf)?)
    }

    async fn write_session_file(
        &self,
        sessions_data: &SessionsData,
    ) -> Result<(), SessionStorageError> {
        let mut db_file = self.db_file.lock().await;
        db_file.set_len(0).await?;
        db_file.rewind().await?;
        db_file.write_all(
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
