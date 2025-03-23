use std::collections::HashMap;
use std::cmp::min;
use std::fmt::{Debug, Formatter};
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};

use tokio::io;
use tokio::io::{AsyncRead, ReadBuf};
use tokio::fs::ReadDir;
use async_trait::async_trait;

use crate::storage::internal::io_trait::{Metadata, NoteStorageIo};
use crate::storage::internal::tests::data::DEFAULT_SPECS;

pub enum FileSpec {
    Dir,
    MetadataError(Box<dyn Sync + Fn() -> io::Error>),
    File {
        contents: Vec<u8>,
    },
    NotEnoughPermsDir,
    OtherOwnerDir,
}

impl FileSpec {
    pub fn empty_file() -> Self {
        FileSpec::File {
            contents: Vec::new(),
        }
    }
}

struct TestFile {
    contents: Vec<u8>,
    position: usize,
    lock: AtomicBool,
}

impl TestFile {
    fn new(contents: impl AsRef<[u8]>) -> Self {
        TestFile {
            contents: contents.as_ref().to_vec(),
            position: 0,
            lock: AtomicBool::new(false),
        }
    }
}

impl AsyncRead for TestFile {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        while this.lock.compare_exchange_weak(
            false,
            true,
            Ordering::Acquire,
            Ordering::Relaxed
        ).is_err() {}
        let position = this.position;
        let remaining_data_size = this.contents.len() - position;
        let to_write = min(remaining_data_size, buf.remaining());
        let end_position = position + to_write;
        buf.put_slice(&this.contents[position .. position + to_write]);
        this.position = end_position;
        this.lock
            .compare_exchange(
                true,
                false,
                Ordering::Release,
                Ordering::Relaxed
            )
            .expect("the lock was supposed to be held");
        Poll::Ready(Ok(()))
    }
}

pub struct TestStorageIo<'a> {
    files: &'a HashMap<String, FileSpec>,
}

impl TestStorageIo<'_> {
    pub fn new() -> Self {
        TestStorageIo {
            files: &DEFAULT_SPECS,
        }
    }

    fn get_spec(&self, path: impl AsRef<Path>) -> &FileSpec {
        self.files.get(path.as_ref().to_str().unwrap()).unwrap().to_owned()
    }
}

impl Debug for TestStorageIo<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
       f.write_str("TestStorageIo")
    }
}

#[async_trait(?Send)]
impl NoteStorageIo for TestStorageIo<'_> {
    async fn metadata(&self, path: impl AsRef<Path>) -> io::Result<Metadata> {
        match self.get_spec(path) {
            FileSpec::Dir => Ok(Metadata { is_dir: true, uid: 1, mode: 0o700 }),
            FileSpec::MetadataError(err) => Err(err()),
            FileSpec::File {..} => Ok(Metadata { is_dir: false, uid: 1, mode: 0o700 }),
            FileSpec::NotEnoughPermsDir => Ok(Metadata { is_dir: false, uid: 1, mode: 0o600 }),
            FileSpec::OtherOwnerDir => Ok(Metadata { is_dir: true, uid: 2, mode: 0o700 }),
        }
    }

    async fn open_file(
        &self,
        path: impl AsRef<Path>,
    ) -> io::Result<(impl AsyncRead + Unpin, u64)> {
        match self.get_spec(path) {
            FileSpec::File { contents } => Ok(
                (TestFile::new(contents), contents.len() as u64)
            ),
            _ => unreachable!()
        }
    }

    async fn write_file(
        &self,
        path: impl AsRef<Path>,
        data: impl AsRef<[u8]>,
    ) -> io::Result<()> {
        todo!()
    }

    async fn rename_file(
        &self,
        from: impl AsRef<Path>,
        to: impl AsRef<Path>,
    ) -> io::Result<()> {
        todo!()
    }

    async fn remove_file(&self, path: impl AsRef<Path>) -> io::Result<()> {
        todo!()
    }

    async fn read_dir(&self, path: impl AsRef<Path>) -> io::Result<ReadDir> {
        todo!()
    }

    fn getuid(&self) -> u32 {
        1
    }
}
