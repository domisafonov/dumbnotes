use std::cmp::min;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::io::Error;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use async_trait::async_trait;
use rand::prelude::StdRng;
use tokio::fs::{read_dir, ReadDir};
use tokio::io;
use tokio::io::{AsyncRead, ReadBuf};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::storage::internal::io_trait::{Metadata, NoteStorageIo};
use crate::storage::internal::rng::{make_uuid, SyncRng};
use crate::storage::internal::tests::data::{DEFAULT_SPECS, RNG};
use crate::storage::internal::{HYPHENED_UUID_SIZE, TMP_FILENAME_INFIX};
use crate::storage::internal::tests::mocks::StorageWrite::Rename;
// TODO: the tests got supercomplicated, rewrite with a full-tree storage
//  emulation, using injected rng's determinism to hardcode test uuids

pub struct VersionedFileSpec {
    pub current_version: AtomicUsize,
    pub specs: Vec<Arc<FileSpec>>,
    pub is_tmp: bool,
}

impl VersionedFileSpec {
    pub fn new(spec: FileSpec, is_tmp: bool) -> Self {
        VersionedFileSpec {
            current_version: AtomicUsize::new(usize::MAX),
            specs: vec![Arc::new(spec)],
            is_tmp,
        }
    }

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

impl Clone for VersionedFileSpec {
    fn clone(&self) -> Self {
        VersionedFileSpec {
            current_version: AtomicUsize::new(self.current_version.load(Ordering::Relaxed)),
            specs: self.specs.clone(),
            is_tmp: self.is_tmp,
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
    EmptyDir,
    CantOpen,
    CantRead,
    CantWrite,
    WriteTmpFile,
    RenameWrittenTmpFile {
        path: String,
        rename_to: String,
    },
    CantRename {
        path: String,
        rename_to: String,
    },
    Remove {
        should_be_written: bool,
    },
    CantRemove,
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
    rng: SyncRng<StdRng>,

    // "[uuid].tmp. to [uuid].tmp.[tmp_uuid]
    // in order of first access
    tmp_files: Mutex<Vec<(String, String)>>,
}

impl TestStorageIo {
    pub fn new() -> Self {
        TestStorageIo {
            files: DEFAULT_SPECS.clone(),
            events: Mutex::new(Vec::new()),
            rng: RNG.clone(),
            tmp_files: Mutex::new(Vec::new()),
        }
    }

    fn get_spec(&self, path: impl AsRef<Path>) -> &FileSpec {
        self.get_versioned_spec(path).get()
    }

    fn get_spec_bumped(&self, path: impl AsRef<Path>) -> &FileSpec {
        let spec = self.get_versioned_spec(path);
        let ret = spec.get();
        spec.bump();
        ret
    }

    fn get_versioned_spec(&self, path: impl AsRef<Path>) -> &VersionedFileSpec {
        let path = path.as_ref().to_str().unwrap();
        let spec = self.files.get(path);
        if spec.is_some() {
            spec.unwrap()
        } else {
            let last_key_ind = path.len() - HYPHENED_UUID_SIZE;
            assert!(last_key_ind >= TMP_FILENAME_INFIX.len(), "can't find spec for path {path}");
            let path = path[0..last_key_ind].to_owned();
            assert!(path.ends_with(TMP_FILENAME_INFIX), "can't find spec for (tmp?) path {path}");
            let spec = self.files.get(&path).unwrap();
            assert!(spec.is_tmp);
            spec
        }
    }

    fn bump_spec(&self, path: impl AsRef<Path>) {
        self.files.get(path.as_ref().to_str().unwrap()).unwrap().bump();
    }

    pub async fn get_events(&self) -> Vec<StorageWrite> {
        self.events.lock().await.to_vec()
    }

    pub async fn get_tmp_files(&self) -> Vec<(String, String)> {
        self.tmp_files.lock().await.to_vec()
    }

    async fn find_write(&self, path: impl AsRef<Path>) -> Option<StorageWrite> {
        self.events.lock().await
            .iter()
            .rfind(|ev|
                matches!(
                    ev,
                    StorageWrite::Write { path: ev_name, .. } if ev_name == path.as_ref()
                )
            )
            .cloned()
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

            FileSpec::WriteTmpFile => {
                let written_to = self.events.lock().await
                    .iter()
                    .rfind(|ev|
                        matches!(
                            ev,
                            StorageWrite::Write { path, .. }
                        )
                    )
                    .is_some();
                if written_to {
                    Ok(Metadata { is_dir: false, uid: 1, mode: 0o700 })
                } else {
                    Err(io::Error::from(io::ErrorKind::NotFound))
                }
            },

            _ => unreachable!()
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
        self.events.lock().await
            .push(
                StorageWrite::Write {
                    path: path.as_ref().to_owned(),
                    data: data.as_ref().to_vec(),
                }
            );
        match self.get_spec_bumped(&path) {
            FileSpec::WriteTmpFile => {
                let mut tmp_files = self.tmp_files.lock().await;
                let tmp_file = tmp_files
                    .iter()
                    .find(|(key, name)|
                        Path::new(name) == path.as_ref()
                    )
                    .map(|(_, name)| name);
                if tmp_file.is_none() {
                    tmp_files.push((
                        path.as_ref().to_str().unwrap()
                            .rsplit_once('.').unwrap().0.to_owned() + ".",
                        path.as_ref().to_str().unwrap().to_owned(),
                    ))
                }
                Ok(())
            },
            FileSpec::CantWrite => Err(Error::from(io::ErrorKind::StorageFull)),
            _ => unreachable!()
        }
    }

    async fn rename_file(
        &self,
        from: impl AsRef<Path>,
        to: impl AsRef<Path>,
    ) -> io::Result<()> {
        self.events.lock().await
            .push(
                Rename {
                    from: from.as_ref().to_owned(),
                    to: to.as_ref().to_owned(),
                }
            );
        match self.get_spec_bumped(&from) {
            FileSpec::RenameWrittenTmpFile { rename_to, .. } => {
                self.find_write(&from)
                    .await
                    .expect("file path was written to before renaming");
                assert_eq!(rename_to, to.as_ref().to_str().unwrap());
                Ok(())
            },
            FileSpec::CantRename { .. } => Err(Error::from(io::ErrorKind::Other)),
            _ => unreachable!()
        }
    }

    async fn remove_file(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.events.lock().await
            .push(
                StorageWrite::Remove {
                    path: path.as_ref().to_owned(),
                }
            );
        match self.get_spec_bumped(&path) {
            FileSpec::Remove { should_be_written } => {
                if *should_be_written {
                    self.find_write(&path)
                        .await
                        .expect("file path was written to before removing");
                }
                Ok(())
            }
            FileSpec::CantRemove => Err(io::Error::from(io::ErrorKind::Other)),
            _ => unreachable!()
        }
    }

    async fn read_dir(&self, path: impl AsRef<Path>) -> io::Result<ReadDir> {
        match self.get_spec(path) {
            FileSpec::EmptyDir => read_dir("/var/empty").await,
            FileSpec::NotEnoughPermsDir => Err(io::Error::from(io::ErrorKind::PermissionDenied)),
            _ => unreachable!()
        }
    }

    fn getuid(&self) -> u32 {
        1
    }

    fn generate_uuid(&self) -> Uuid {
        make_uuid(&mut self.rng.lock().unwrap())
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
