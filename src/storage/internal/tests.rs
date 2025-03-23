// TODO: remember to test errors being logged

use std::cmp::min;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};

use async_trait::async_trait;
use lazy_static::lazy_static;
use tokio::fs::ReadDir;
use tokio::io;
use tokio::io::{AsyncRead, ReadBuf};

use super::*;

#[tokio::test]
async fn create_storage_ok() {
    let io = TestStorageIo::new();
    NoteStorageImpl::new_internal("/", io).await
        .expect("successful storage creation");
}

#[tokio::test]
async fn create_storage_metadata_fail() {
    let io = TestStorageIo::new();
    let err = NoteStorageImpl::new_internal("/meta_fail", io)
        .await.expect_err("should fail");
    match err {
        StorageError::IoError(_) => (),
        e => panic!("wrong error type: #{e:?}"),
    }
}

#[tokio::test]
async fn create_storage_not_is_dir() {
    create_storage_dir_does_not_exist_error().await
}

#[tokio::test]
async fn create_storage_dir_does_not_exist() {
    create_storage_dir_does_not_exist_error().await
}

async fn create_storage_dir_does_not_exist_error() {
    let io = TestStorageIo::new();
    let err = NoteStorageImpl::new_internal("/a_file", io)
        .await.expect_err("should fail");
    match err {
        StorageError::DoesNotExist => (),
        e => panic!("wrong error type: #{e:?}"),
    }
}

#[tokio::test]
async fn create_storage_wrong_permissions() {
    let io = TestStorageIo::new();
    let err = NoteStorageImpl::new_internal("/other_owner_dir", io)
        .await.expect_err("should fail");
    match err {
        StorageError::PermissionError => (),
        e => panic!("wrong error type: #{e:?}"),
    }
}

#[tokio::test]
async fn read_note_normal() {
    let io = TestStorageIo::new();
    let mut storage = NoteStorageImpl::new_internal("/", io)
        .await.expect("successful storage creation");
    let note = storage.read_note(
        &UsernameString::from_str("read_note").unwrap(),
        *READ_NOTE_NORMAL_UUID
    ).await.expect("successful note read");
    assert_eq!(note.id, *READ_NOTE_NORMAL_UUID);
    assert_eq!(note.name, Some("normal title".into()));
    assert_eq!(note.contents, "normal contents");
}

#[tokio::test]
async fn read_note_empty_file() {
    todo!()
}

#[tokio::test]
async fn read_note_empty_name() {
    todo!()
}

#[tokio::test]
async fn read_note_empty_contents() {
    todo!()
}

#[tokio::test]
async fn read_note_cant_read() {
    todo!()
}

#[tokio::test]
async fn read_note_invalid_utf8() {
    todo!()
}

#[tokio::test]
async fn read_note_file_too_big() {
    todo!()
}

#[tokio::test]
async fn read_note_name_too_big() {
    todo!()
}

#[tokio::test]
async fn read_note_name_not_terminated_with_newline() {
    // incorrect format, but must still work
    todo!()
}

#[tokio::test]
async fn read_note_file_became_too_big_after_metadata_read() {
    todo!()
}

#[tokio::test]
async fn write_note_normal() {
    todo!()
}

#[tokio::test]
async fn write_note_write_empty_name() {
    todo!()
}

#[tokio::test]
async fn write_note_write_empty_contents() {
    todo!()
}

#[tokio::test]
async fn write_note_write_empty_name_and_contents() {
    todo!()
}

#[tokio::test]
async fn write_note_write_error() {
    todo!()
}

#[tokio::test]
async fn write_note_rename_error() {
    // also, error on temporary file removal
    todo!()
}

// TODO: list_notes, get_note_details, delete_note

struct TestStorageIo {
    files: HashMap<String, FileSpec>,
}

lazy_static!(
    static ref READ_NOTE_NORMAL_UUID: Uuid = Uuid::new_v4();
);

impl TestStorageIo {
    fn new() -> Self {
        TestStorageIo {
            files: HashMap::from([
                ("/".into(), FileSpec::Dir),
                ("/meta_fail".into(), 
                    FileSpec::MetadataError(
                        Box::new(|| io::Error::from(io::ErrorKind::StorageFull))
                    )
                ),
                ("/a_file".into(), FileSpec::empty_file()),
                ("/no_such_dir".into(), 
                    FileSpec::MetadataError(
                        Box::new(|| io::Error::from(io::ErrorKind::NotFound))
                    )
                ),
                ("/not_enough_perms_dir".into(), FileSpec::NotEnoughPermsDir),
                ("/other_owner_dir".into(), FileSpec::OtherOwnerDir),
                ("/note_dir".into(), FileSpec::Dir),
                ("/read_note".into(), FileSpec::Dir),
                ("/read_note/".to_string() + &READ_NOTE_NORMAL_UUID.hyphenated().to_string(),
                    FileSpec::File {
                       contents: "normal title\nnormal contents".as_bytes().into(),
                    },
                ),
            ]),
        }
    }

    fn get_spec(&self, path: impl AsRef<Path>) -> &FileSpec {
        self.files.get(path.as_ref().to_str().unwrap()).unwrap().to_owned()
    }
}

impl Debug for TestStorageIo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
       f.write_str("TestStorageIo") 
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

enum FileSpec {
    Dir,
    MetadataError(Box<dyn Send + Fn() -> io::Error>),
    File {
        contents: Vec<u8>,
    },
    NotEnoughPermsDir,
    OtherOwnerDir,
}

impl FileSpec {
    fn empty_file() -> Self {
        FileSpec::File {
            contents: Vec::new(),
        }
    }
}

#[async_trait(?Send)]
impl NoteStorageIo for TestStorageIo {
    async fn metadata(&self, path: impl AsRef<Path>) -> io::Result<Metadata> {
        match self.get_spec(path) {
            FileSpec::Dir => Ok(Metadata { is_dir: true, uid: 1, mode: 0o700 }),
            FileSpec::MetadataError(err) => Err(err()),
            FileSpec::File {..} => Ok(Metadata { is_dir: false, uid: 1, mode: 0o700 }),
            FileSpec::NotEnoughPermsDir => Ok(Metadata { is_dir: false, uid: 1, mode: 0o600 }),
            FileSpec::OtherOwnerDir => Ok(Metadata { is_dir: true, uid: 2, mode: 0o700 }),
        }
    }

    async fn open_file(&self, path: impl AsRef<Path>) -> io::Result<(impl AsyncRead + Unpin, u64)> {
        match self.get_spec(path) {
            FileSpec::File { contents } => Ok(
                (TestFile::new(contents), contents.len() as u64)
            ),
            _ => unreachable!()
        }
    }

    async fn write_file(&self, path: impl AsRef<Path>, data: impl AsRef<[u8]>) -> io::Result<()> {
        todo!()
    }

    async fn rename_file(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
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

impl AsyncRead for TestFile {
    fn poll_read(
        self: Pin<&mut Self>, 
        cx: &mut Context<'_>, 
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        while this.lock.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {}
        let position = this.position;
        let remaining_data_size = this.contents.len() - position;
        let to_write = min(remaining_data_size, buf.remaining());
        let end_position = position + to_write;
        buf.put_slice(&this.contents[position .. position + to_write]);
        this.position = end_position;
        this.lock.compare_exchange(true, false, Ordering::Release, Ordering::Relaxed)
            .expect("the lock was supposed to be held");
        Poll::Ready(Ok(()))
    }
}
