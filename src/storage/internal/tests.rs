// TODO: remember to test errors being logged

use std::io::Read;
use mocks::TestStorageIo;
use crate::storage::internal::tests::data::*;
use crate::storage::internal::tests::mocks::StorageWrite;
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
    assert!(matches!(err, StorageError::IoError(_)), "wrong error type: {err:#?}");
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
    assert!(matches!(err, StorageError::DoesNotExist), "wrong error type: {err:#?}");
}

#[tokio::test]
async fn create_storage_wrong_permissions() {
    let io = TestStorageIo::new();
    let err = NoteStorageImpl::new_internal("/other_owner_dir", io)
        .await.expect_err("should fail");
    assert!(matches!(err, StorageError::PermissionError), "wrong error type: {err:#?}");
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
async fn read_note_cant_open() {
    read_note_with_error(io::ErrorKind::Other, *READ_NOTE_CANT_OPEN_UUID).await
}

#[tokio::test]
async fn read_note_cant_read() {
    read_note_with_error(io::ErrorKind::BrokenPipe, *READ_NOTE_CANT_READ_UUID).await
}

#[tokio::test]
async fn read_note_invalid_utf8() {
    let note = read_note_successfully(*READ_NOTE_INVALID_UTF8_UUID).await;
    assert_eq!(note.name, Some(READ_NOTE_INVALID_UTF8_TITLE.into()));
    assert_eq!(note.contents, READ_NOTE_INVALID_UTF8_CONTENTS);
}

#[tokio::test]
async fn read_note_file_too_big() {
    todo!("implement the test after the config is done")
}

#[tokio::test]
async fn read_note_name_too_big() {
    todo!("implement the test after the config is done")
}

#[tokio::test]
async fn read_note_name_not_terminated_with_newline() {
    let note = read_note_successfully(*READ_NOTE_NO_NEWLINE_UUID).await;
    assert_eq!(note.name, Some("normal title".into()));
    assert_eq!(note.contents, "");
}

#[tokio::test]
async fn read_note_file_became_too_big_after_metadata_read() {
    todo!("implement the test after the config is done")
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

async fn read_note_with_error(error_kind: io::ErrorKind, uuid: Uuid) {
    let io = TestStorageIo::new();
    let mut storage = NoteStorageImpl::new_internal("/", io)
        .await.expect("successful storage creation");
    let err = storage.read_note(
        &UsernameString::from_str("read_note").unwrap(),
        uuid,
    ).await.expect_err("should fail");
    match err {
        StorageError::IoError(e) if e.kind() == error_kind => (),
        e => panic!("wrong error type: {e:#?}"),
    }
}

#[tokio::test]
async fn write_note_normal() {
    write_note_normal_impl(Some("normal title"), "normal contents").await;
}

#[tokio::test]
async fn write_note_write_empty_name() {
    write_note_normal_impl(None, "normal contents").await;
}

#[tokio::test]
async fn write_note_write_empty_contents() {
    write_note_normal_impl(Some("normal title"), "").await;
}

async fn write_note_normal_impl(title: Option<&str>, contents: &str) {
    let io = TestStorageIo::new();
    let mut storage = NoteStorageImpl::new_internal("/", io)
        .await.expect("successful storage creation");
    storage.write_note(
        &UsernameString::from_str("write_note").unwrap(),
        &Note {
            id: *WRITE_NOTE_NORMAL_UUID,
            name: title.map(|t| t.into()),
            contents: contents.into(),
        },
    ).await.expect("successful write");
    let events = storage.io.get_events().await;
    assert_eq!(events.len(), 1);
    let title = title.map(String::from).unwrap_or("".into()) + "\n";
    assert_eq!(
        events[0],
        StorageWrite::Write {
            path: make_tmp_path("/write_note", *WRITE_NOTE_NORMAL_UUID).into(),
            data: format!("{title}{contents}").into(),
        },
    );
}

#[tokio::test]
async fn write_note_write_empty_name_and_contents() {
    let io = TestStorageIo::new();
    let mut storage = NoteStorageImpl::new_internal("/", io)
        .await.expect("successful storage creation");
    storage.write_note(
        &UsernameString::from_str("write_note").unwrap(),
        &Note {
            id: *WRITE_NOTE_NORMAL_UUID,
            name: None,
            contents: "".into(),
        },
    ).await.expect("successful write");
    
    let events = storage.io.get_events().await;
    assert_eq!(events.len(), 1);
    
    let (path, data) = match events[0] {
        StorageWrite::Write { ref path, ref data } => (path, data),
        _ => panic!("not a write event: {:?}", events[0]),
    };
    assert_eq!(
        *path,
        PathBuf::from(
            make_tmp_path("/write_note", *WRITE_NOTE_NORMAL_UUID)
        )
    );
    assert!(*data == "\n".bytes().collect::<Vec<_>>() || data.is_empty());
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
