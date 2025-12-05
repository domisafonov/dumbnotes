use crate::rng::make_uuid;
use async_trait::async_trait;
use std::os::unix::prelude::*;
use std::path::Path;
use libc::{gid_t, mode_t, uid_t};
use tokio::{fs, io};
use uuid::Uuid;
use crate::util::get_ids;

#[async_trait]
pub trait NoteStorageIo: Send + Sync
{
    async fn metadata(
        &self,
        path: impl AsRef<Path> + Send,
    ) -> io::Result<Metadata>;

    // TODO: https://github.com/rust-lang/rust/issues/130113
    //  when fixed, it would be a regular async fn
    fn open_file(
        &self,
        path: impl AsRef<Path> + Send,
    ) -> impl Future<Output=io::Result<OpenFile<impl io::AsyncRead + Unpin + Send + Sync>>> + Send;

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

    fn get_ids(&self) -> (uid_t, gid_t);
    
    fn generate_uuid(&self) -> Uuid;
}

pub struct Metadata {
    pub is_dir: bool,
    pub uid: uid_t,
    pub gid: gid_t,
    pub mode: mode_t,
}

pub struct OpenFile<F: io::AsyncRead + Unpin + Send + Sync> {
    pub file: F,
    pub size: u64,
    pub mtime: i64,
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
            gid: meta.gid(),
            mode: meta.mode() as mode_t,
        })
    }

    fn open_file(
        &self,
        path: impl AsRef<Path> + Send,
    ) -> impl Future<Output=io::Result<OpenFile<impl io::AsyncRead + Unpin + Send + Sync>>> + Send { async move {
        let file = fs::File::open(path).await?;
        let metadata = file.metadata().await?;
        Ok(
            OpenFile {
                file,
                size: metadata.len(),
                mtime: metadata.mtime(),
            }
        )
    } }

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

    fn get_ids(&self) -> (uid_t, gid_t) {
        get_ids()
    }
    
    fn generate_uuid(&self) -> Uuid {
        make_uuid(&mut rand::rng())
    }
}
