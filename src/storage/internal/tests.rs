// TODO: remember to test errors being logged

use data::TestStorageIo;
use crate::storage::internal::tests::data::*;
use super::*;

mod data;

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
    let io = TestStorageIo::new();
    let mut storage = NoteStorageImpl::new_internal("/", io)
        .await.expect("successful storage creation");
    let note = storage.read_note(
        &UsernameString::from_str("read_note").unwrap(),
        *READ_NOTE_EMPTY_UUID
    ).await.expect("successful note read");
    assert_eq!(note.id, *READ_NOTE_EMPTY_UUID);
    assert_eq!(note.name, None);
    assert_eq!(note.contents, "");
}

#[tokio::test]
async fn read_note_empty_name() {
    let io = TestStorageIo::new();
    let mut storage = NoteStorageImpl::new_internal("/", io)
        .await.expect("successful storage creation");
    let note = storage.read_note(
        &UsernameString::from_str("read_note").unwrap(),
        *READ_NOTE_EMPTY_NAME_UUID
    ).await.expect("successful note read");
    assert_eq!(note.id, *READ_NOTE_EMPTY_NAME_UUID);
    assert_eq!(note.name, None);
    assert_eq!(note.contents, "normal contents");
}

#[tokio::test]
async fn read_note_empty_contents() {
    let io = TestStorageIo::new();
    let mut storage = NoteStorageImpl::new_internal("/", io)
        .await.expect("successful storage creation");
    let note = storage.read_note(
        &UsernameString::from_str("read_note").unwrap(),
        *READ_NOTE_EMPTY_CONTENTS_UUID
    ).await.expect("successful note read");
    assert_eq!(note.id, *READ_NOTE_EMPTY_CONTENTS_UUID);
    assert_eq!(note.name, Some("normal title".into()));
    assert_eq!(note.contents, "");
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