// TODO: remember to test errors being logged

use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_trait::async_trait;
use tokio::io;
use tokio::io::{AsyncRead, ReadBuf};

use super::*;

#[tokio::test]
async fn create_storage_ok() {
    let io = TestStorageIo::new();
    NoteStorageImpl::new_internal("/", io).await.expect("successful storage creation");
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
        StorageError::DirectoryDoesNotExist => (),
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
    todo!()
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

struct TestStorageIo {
    files: HashMap<String, FileSpec>,
}

impl TestStorageIo {
    fn new() -> Self {
        TestStorageIo {
            files: HashMap::from([
                ("/".into(), FileSpec::Dir(None)),
                ("/meta_fail".into(), 
                    FileSpec::MetadataError(
                        Box::new(|| io::Error::from(io::ErrorKind::StorageFull))
                    )
                ),
                ("/a_file".into(), FileSpec::File),
                ("/no_such_dir".into(), 
                    FileSpec::MetadataError(
                        Box::new(|| io::Error::from(io::ErrorKind::NotFound))
                    )
                ),
                ("/not_enough_perms_dir".into(), FileSpec::NotEnoughPermsDir),
                ("/other_owner_dir".into(), FileSpec::OtherOwnerDir),
            ]),
        }
    }
}

impl Debug for TestStorageIo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
       f.write_str("TestStorageIo") 
    }
}

struct TestFile {
    
}

enum FileSpec {
    Dir(Option<Box<FileSpec>>),
    MetadataError(Box<dyn Send + Fn() -> io::Error>),
    File,
    NotEnoughPermsDir,
    OtherOwnerDir,
}

#[async_trait(?Send)]
impl NoteStorageIo for TestStorageIo {
    async fn metadata(&mut self, path: impl AsRef<Path>) -> io::Result<Metadata> {
        match self.files.get(path.as_ref().to_str().unwrap()).unwrap() {
            FileSpec::Dir(_) => Ok(Metadata { is_dir: true, uid: 1, mode: 0o700 }),
            FileSpec::MetadataError(err) => Err(err()),
            FileSpec::File => Ok(Metadata { is_dir: false, uid: 1, mode: 0o700 }),
            FileSpec::NotEnoughPermsDir => Ok(Metadata { is_dir: false, uid: 1, mode: 0o600 }),
            FileSpec::OtherOwnerDir => Ok(Metadata { is_dir: true, uid: 2, mode: 0o700 }),
        }
    }

    async fn open_file(&mut self, path: impl AsRef<Path>) -> io::Result<(impl AsyncRead + Unpin, u64)> {
        Ok(
            (TestFile {}, 0)
        )
    }

    async fn write_file(&mut self, path: impl AsRef<Path>, data: impl AsRef<[u8]>) -> io::Result<()> {
        todo!()
    }

    async fn rename_file(&mut self, from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
        todo!()
    }

    async fn remove_file(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        todo!()
    }

    fn getuid(&self) -> u32 {
        1
    }
}

impl AsyncRead for TestFile {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        todo!()
    }
}
