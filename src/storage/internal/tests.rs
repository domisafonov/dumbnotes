// TODO: remember to test errors being logged

use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_trait::async_trait;
use tokio::io;
use tokio::io::{AsyncRead, ReadBuf};

use super::*;

#[tokio::test]
async fn create_storage_ok() {
    let io = TestStorageIo {};
    NoteStorageImpl::new_internal("/", io).await.expect("successful storage creation");
}

#[tokio::test]
async fn create_storage_metadata_fail() {
    todo!()
}

#[tokio::test]
async fn create_storage_not_is_dir() {
    todo!()
}

#[tokio::test]
async fn create_storage_dir_does_not_exist() {
    todo!()
}

#[tokio::test]
async fn create_storage_wrong_permissions() {
    todo!()
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
    
}

struct TestFile {
    
}

#[async_trait(?Send)]
impl NoteStorageIo for TestStorageIo {
    async fn metadata(&mut self, path: impl AsRef<Path>) -> io::Result<Metadata> {
        Ok(Metadata { is_dir: true, uid: 1, mode: 0o700 })
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
