use crate::rng::make_uuid;
use async_trait::async_trait;
use std::os::unix::prelude::*;
use std::path::Path;
use tokio::{fs, io};
use uuid::Uuid;

#[async_trait]
pub trait NoteStorageIo: Send + Sync {
    async fn metadata(
        &self,
        path: impl AsRef<Path> + Send,
    ) -> io::Result<Metadata>;

    async fn open_file(
        &self,
        path: impl AsRef<Path> + Send,
    ) -> io::Result<(impl io::AsyncRead + Unpin + Send + Sync, u64)>;

    async fn write_file(
        &self,
        path: impl AsRef<Path> + Send,
        data: impl AsRef<[u8]> + Send,
    ) -> io::Result<()>;

    async fn rename_file(
        &self,
        from: impl AsRef<Path> + Send,
        to: impl AsRef<Path> + Send,
    ) -> io::Result<()>;

    async fn remove_file(
        &self,
        path: impl AsRef<Path> + Send,
    ) -> io::Result<()>;
    
    // TODO: get ReadDir behind a facade to make it properly testable
    async fn read_dir(
        &self,
        path: impl AsRef<Path> + Send,
    ) -> io::Result<fs::ReadDir>;

    fn getuid(&self) -> u32;
    
    fn generate_uuid(&self) -> Uuid;
}

pub struct Metadata {
    pub is_dir: bool,
    pub uid: u32,
    pub mode: u32,
}

pub struct ProductionNoteStorageIo;

impl ProductionNoteStorageIo {
    pub fn new() -> Self {
        ProductionNoteStorageIo
    }
}

#[async_trait]
impl NoteStorageIo for ProductionNoteStorageIo {
    async fn metadata(
        &self, 
        path: impl AsRef<Path> + Send,
    ) -> io::Result<Metadata> {
        let meta = fs::metadata(path).await?;
        Ok(Metadata {
            is_dir: meta.is_dir(),
            uid: meta.uid(),
            mode: meta.mode(),
        })
    }

    async fn open_file(
        &self,
        path: impl AsRef<Path> + Send,
    ) -> io::Result<(impl io::AsyncRead + Unpin, u64)> {
        let file = fs::File::open(path).await?;
        let metadata = file.metadata().await?;
        Ok((file, metadata.len()))
    }

    async fn write_file(
        &self,
        path: impl AsRef<Path> + Send,
        data: impl AsRef<[u8]> + Send,
    ) -> io::Result<()> {
        fs::write(path, data).await
    }

    async fn rename_file(
        &self,
        from: impl AsRef<Path> + Send,
        to: impl AsRef<Path> + Send,
    ) -> io::Result<()> {
        fs::rename(from, to).await
    }

    async fn remove_file(
        &self,
        path: impl AsRef<Path> + Send,
    ) -> io::Result<()> {
        fs::remove_file(path).await
    }
    
    async fn read_dir(
        &self,
        path: impl AsRef<Path> + Send,
    ) -> io::Result<fs::ReadDir> {
        fs::read_dir(path).await
    }

    fn getuid(&self) -> u32 {
        // SAFETY: a libc call
        unsafe { libc::getuid() }
    }
    
    fn generate_uuid(&self) -> Uuid {
        make_uuid(&mut rand::rng())
    }
}
