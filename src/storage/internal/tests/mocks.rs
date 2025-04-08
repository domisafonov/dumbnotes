use std::collections::HashMap;
use std::cmp::min;
use std::fmt::{Debug, Formatter};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::task::{Context, Poll};

use tokio::io;
use tokio::io::{AsyncRead, ReadBuf};
use tokio::fs::ReadDir;
use async_trait::async_trait;
use tokio::sync::Mutex;
use crate::storage::internal::io_trait::{Metadata, NoteStorageIo};
use crate::storage::internal::tests::data::DEFAULT_SPECS;

pub struct VersionedFileSpec {
    pub current_version: AtomicUsize,
    pub specs: Vec<Arc<FileSpec>>,
}

impl VersionedFileSpec {
    pub fn get(&self) -> &FileSpec {
        let current_version = self.current_version.load(Ordering::Relaxed);
        &self.specs[
            if current_version == usize::MAX {
                0
            } else {
                current_version
            }
        ]
    }
    
    pub fn bump(&self) {
        if self.current_version.load(Ordering::Relaxed) != usize::MAX {
            self.current_version.fetch_add(1, Ordering::Relaxed);
        }
    }
}

impl From<FileSpec> for VersionedFileSpec {
    fn from(spec: FileSpec) -> Self {
        VersionedFileSpec {
            current_version: AtomicUsize::new(usize::MAX),
            specs: vec![Arc::new(spec)],
        }
    }
}

impl Clone for VersionedFileSpec {
    fn clone(&self) -> Self {
        VersionedFileSpec {
            current_version: AtomicUsize::new(self.current_version.load(Ordering::Relaxed)),
            specs: self.specs.clone(),
        }
    }
}

pub enum FileSpec {
    Dir,
    MetadataError(Box<dyn Sync + Send + Fn() -> io::Error>),
    File {
        contents: Vec<u8>,
    },
    NotEnoughPermsDir,
    OtherOwnerDir,
    CantOpen,
    CantRead,
    WriteFile,
    RenameWrittenFile {
        path: String,
        rename_to: String,
    }
}

impl FileSpec {
    pub fn empty_file() -> Self {
        FileSpec::File {
            contents: Vec::new(),
        }
    }
}

enum TestFile {
    File(TestFileFile),
    CantRead,
}

struct TestFileFile {
    contents: Vec<u8>,
    position: usize,
    lock: AtomicBool,
}

impl TestFile {
    fn new(contents: impl AsRef<[u8]>) -> Self {
        TestFile::File(TestFileFile {
            contents: contents.as_ref().to_vec(),
            position: 0,
            lock: AtomicBool::new(false),
        })
    }
}

impl AsyncRead for TestFile {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = match self.get_mut() {
            TestFile::File(file) => file,
            TestFile::CantRead => return Poll::Ready(
                Err(io::Error::from(io::ErrorKind::BrokenPipe))
            ),
        };

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

pub struct TestStorageIo {
    files: HashMap<String, VersionedFileSpec>,
    events: Mutex<Vec<StorageWrite>>,
}

impl TestStorageIo {
    pub fn new() -> Self {
        TestStorageIo {
            files: DEFAULT_SPECS.clone(),
            events: Mutex::new(Vec::new()),
        }
    }

    fn get_spec(&self, path: impl AsRef<Path>) -> &FileSpec {
        self.files.get(path.as_ref().to_str().unwrap()).unwrap().get()
    }
    
    fn get_spec_bumped(&self, path: impl AsRef<Path>) -> &FileSpec {
        let spec = self.files
            .get(path.as_ref().to_str().unwrap())
            .unwrap();
        let ret = spec.get();
        spec.bump();
        ret
    }
    
    fn bump_spec(&self, path: impl AsRef<Path>) {
        self.files.get(path.as_ref().to_str().unwrap()).unwrap().bump();
    }

    pub async fn get_events(&self) -> Vec<StorageWrite> {
        self.events.lock().await.to_vec()
    }
}

impl Debug for TestStorageIo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
       f.write_str("TestStorageIo")
    }
}

#[async_trait(?Send)]
impl NoteStorageIo for TestStorageIo {
    async fn metadata(&self, path: impl AsRef<Path>) -> io::Result<Metadata> {
        match self.get_spec(path) {
            FileSpec::Dir => Ok(Metadata { is_dir: true, uid: 1, mode: 0o700 }),
            FileSpec::MetadataError(err) => Err(err()),
            FileSpec::NotEnoughPermsDir => Ok(Metadata { is_dir: false, uid: 1, mode: 0o600 }),
            FileSpec::OtherOwnerDir => Ok(Metadata { is_dir: true, uid: 2, mode: 0o700 }),

            FileSpec::File {..}
                | FileSpec::CantOpen
                | FileSpec::CantRead
            => Ok(Metadata { is_dir: false, uid: 1, mode: 0o700 }),

            FileSpec::WriteFile => Ok(Metadata { is_dir: false, uid: 1, mode: 0o700 }),
            FileSpec::RenameWrittenFile { .. } => todo!(),
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
            FileSpec::CantOpen => Err(io::Error::from(io::ErrorKind::Other)),
            FileSpec::CantRead => Ok((TestFile::CantRead, 0)),
            _ => unreachable!()
        }
    }

    async fn write_file(
        &self,
        path: impl AsRef<Path>,
        data: impl AsRef<[u8]>,
    ) -> io::Result<()> {
        match self.get_spec_bumped(&path) {
            FileSpec::WriteFile => {
                self.events.lock().await
                    .push(
                        StorageWrite::Write {
                            path: path.as_ref().to_owned(),
                            data: data.as_ref().to_vec(),
                        }
                    );
                Ok(())
            },
            _ => unreachable!()
        }
    }

    async fn rename_file(
        &self,
        from: impl AsRef<Path>,
        to: impl AsRef<Path>,
    ) -> io::Result<()> {
        match self.get_spec(&from) {
            FileSpec::RenameWrittenFile { path, rename_to } => {
                let write_event = self.events.lock().await
                    .iter()
                    .rfind(|ev|
                        matches!(ev, StorageWrite::Write { path: from, .. })
                    )
                    .expect("file path was written to before renaming");
                assert_eq!(rename_to, to.as_ref().to_str().unwrap());
                Ok(())
            },
            _ => unreachable!()
        }
    }

    async fn remove_file(&self, path: impl AsRef<Path>) -> io::Result<()> {
        match self.get_spec(&path) {
            _ => unreachable!()
        }
    }

    async fn read_dir(&self, path: impl AsRef<Path>) -> io::Result<ReadDir> {
        todo!()
    }

    fn getuid(&self) -> u32 {
        1
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageWrite {
    Write {
        path: PathBuf,
        data: Vec<u8>,
    },
    Rename {
        from: PathBuf,
        to: PathBuf,
    },
    Remove {
        path: PathBuf,
    },
}
