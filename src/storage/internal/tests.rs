// TODO: remember to test errors being logged

use mocks::TestStorageIo;
use crate::storage::internal::tests::data::*;
use super::*;

mod data;
mod mocks;

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
        e => panic!("wrong error type: {e:#?}"),
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
        e => panic!("wrong error type: {e:#?}"),
    }
}

#[tokio::test]
async fn create_storage_wrong_permissions() {
    let io = TestStorageIo::new();
    let err = NoteStorageImpl::new_internal("/other_owner_dir", io)
        .await.expect_err("should fail");
    match err {
        StorageError::PermissionError => (),
        e => panic!("wrong error type: {e:#?}"),
    }
}

#[tokio::test]
async fn read_note_normal() {
    let note = read_note_successfully(*READ_NOTE_NORMAL_UUID).await;
    assert_eq!(note.name, Some("normal title".into()));
    assert_eq!(note.contents, "normal contents");
}

#[tokio::test]
async fn read_note_empty_file() {
    let note = read_note_successfully(*READ_NOTE_EMPTY_UUID).await;
    assert_eq!(note.name, None);
    assert_eq!(note.contents, "");
}

#[tokio::test]
async fn read_note_empty_name() {
    let note = read_note_successfully(*READ_NOTE_EMPTY_NAME_UUID).await;
    assert_eq!(note.name, None);
    assert_eq!(note.contents, "normal contents");
}

#[tokio::test]
async fn read_note_empty_contents() {
    let note = read_note_successfully(*READ_NOTE_EMPTY_CONTENTS_UUID).await;
    assert_eq!(note.name, Some("normal title".into()));
    assert_eq!(note.contents, "");
}

#[tokio::test]
async fn read_note_cant_read() {
    todo!()
}

#[tokio::test]
async fn read_note_invalid_utf8() {
    let note = read_note_successfully(*READ_NOTE_INVALID_UTF8).await;
    assert_eq!(note.name, Some(READ_NOTE_INVALID_UTF8_TITLE.into()));
    assert_eq!(note.contents, READ_NOTE_INVALID_UTF8_CONTENTS);
}

#[tokio::test]
async fn read_note_file_too_big() {
    todo!() // will implement after the config
}

#[tokio::test]
async fn read_note_name_too_big() {
    todo!() // will implement after the config
}

#[tokio::test]
async fn read_note_name_not_terminated_with_newline() {
    let note = read_note_successfully(*READ_NOTE_NO_NEWLINE_UUID).await;
    assert_eq!(note.name, Some("normal title".into()));
    assert_eq!(note.contents, "");
}

#[tokio::test]
async fn read_note_file_became_too_big_after_metadata_read() {
    todo!()
}

async fn read_note_successfully(id: Uuid) -> Note {
    let io = TestStorageIo::new();
    let mut storage = NoteStorageImpl::new_internal("/", io)
        .await.expect("successful storage creation");
    let note = storage.read_note(
        &UsernameString::from_str("read_note").unwrap(),
        id,
    ).await.expect("successful note read");
    assert_eq!(note.id, id);
    note
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