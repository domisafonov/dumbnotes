use std::path::Path;
use std::os::unix::prelude::*;
use async_trait::async_trait;
use rand::RngCore;
use tokio::{fs, io};
use uuid::Uuid;
use crate::storage::internal::rng::{make_uuid, SyncRng};

#[async_trait(?Send)]
pub trait NoteStorageIo: Send {
    async fn metadata(
        &self,
        path: impl AsRef<Path>,
    ) -> io::Result<Metadata>;

    async fn open_file(
        &self,
        path: impl AsRef<Path>,
    ) -> io::Result<(impl io::AsyncRead + Unpin, u64)>;

    async fn write_file(
        &self,
        path: impl AsRef<Path>,
        data: impl AsRef<[u8]>,
    ) -> io::Result<()>;

    async fn rename_file(
        &self,
        from: impl AsRef<Path>,
        to: impl AsRef<Path>,
    ) -> io::Result<()>;

    async fn remove_file(
        &self,
        path: impl AsRef<Path>,
    ) -> io::Result<()>;
    
    async fn read_dir(
        &self,
        path: impl AsRef<Path>,
    ) -> io::Result<fs::ReadDir>;

    fn getuid(&self) -> u32;
    
    fn generate_uuid(&self) -> Uuid;
}

pub struct Metadata {
    pub is_dir: bool,
    pub uid: u32,
    pub mode: u32,
}

pub struct ProductionNoteStorageIo {
    rng: SyncRng,
}

impl ProductionNoteStorageIo {
    pub fn new<R: RngCore + 'static>(rng: R) -> Self {
        ProductionNoteStorageIo {
            rng: SyncRng::new(rng),
        }
    }
}

#[async_trait(?Send)]
impl NoteStorageIo for ProductionNoteStorageIo {
    async fn metadata(
        &self, 
        path: impl AsRef<Path>,
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
        path: impl AsRef<Path>,
    ) -> io::Result<(impl io::AsyncRead + Unpin, u64)> {
        let file = fs::File::open(path).await?;
        let metadata = file.metadata().await?;
        Ok((file, metadata.len()))
    }

    async fn write_file(
        &self,
        path: impl AsRef<Path>,
        data: impl AsRef<[u8]>,
    ) -> io::Result<()> {
        fs::write(path, data).await
    }

    async fn rename_file(
        &self,
        from: impl AsRef<Path>,
        to: impl AsRef<Path>,
    ) -> io::Result<()> {
        fs::rename(from, to).await
    }

    async fn remove_file(&self, path: impl AsRef<Path>) -> io::Result<()> {
        fs::remove_file(path).await
    }
    
    async fn read_dir(
        &self,
        path: impl AsRef<Path>,
    ) -> io::Result<fs::ReadDir> {
        fs::read_dir(path).await
    }

    fn getuid(&self) -> u32 {
        unsafe { libc::getuid() }
    }
    
    fn generate_uuid(&self) -> Uuid {
        make_uuid(&mut self.rng.rng.lock().unwrap())
    }
}
