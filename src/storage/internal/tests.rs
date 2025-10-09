// TODO: remember to test errors being logged

use mocks::TestStorageIo;
use crate::storage::internal::tests::data::*;
use crate::storage::internal::tests::mocks::StorageWrite;
use super::*;

mod data;
mod mocks;

#[tokio::test]
async fn create_storage_ok() {
    let io = TestStorageIo::new();
    make_default_limits_storage("/", io).await
        .expect("storage creation failed");
}

#[tokio::test]
async fn create_storage_metadata_fail() {
    let io = TestStorageIo::new();
    let err = make_default_limits_storage("/meta_fail", io)
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
    let err = make_default_limits_storage("/a_file", io)
        .await.expect_err("should fail");
    assert!(matches!(err, StorageError::DoesNotExist), "wrong error type: {err:#?}");
}

#[tokio::test]
async fn create_storage_wrong_permissions() {
    let io = TestStorageIo::new();
    let err = make_default_limits_storage("/other_owner_dir", io)
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
    let mut storage = make_default_limits_storage("/", io)
        .await.expect("storage creation failed");
    let note = storage.read_note(
        &UsernameString::from_str("read_note").unwrap(),
        id,
    ).await.expect("note read failed");
    assert_eq!(note.id, id);
    note
}

async fn read_note_with_error(error_kind: io::ErrorKind, uuid: Uuid) {
    let io = TestStorageIo::new();
    let mut storage = make_default_limits_storage("/", io)
        .await.expect("storage creation failed");
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
    let mut storage = make_default_limits_storage("/", io)
        .await.expect("storage creation failed");
    storage.write_note(
        &UsernameString::from_str("write_note").unwrap(),
        &Note {
            id: *WRITE_NOTE_NORMAL_UUID,
            name: title.map(|t| t.into()),
            contents: contents.into(),
        },
    ).await.expect("write failed");
    let tmp_files = storage.io.get_tmp_files().await;
    assert_eq!(tmp_files.len(), 1);
    let (tmp_file_key, tmp_filename) = &tmp_files[0];
    assert_eq!(*tmp_file_key, make_tmp_path("/write_note", *WRITE_NOTE_NORMAL_UUID).path);
    let events = storage.io.get_events().await;
    assert_eq!(events.len(), 2);
    let title = title.map(String::from).unwrap_or("".into()) + "\n";
    assert_eq!(
        events[0],
        StorageWrite::Write {
            path: tmp_filename.into(),
            data: format!("{title}{contents}").into(),
        },
    );
}

#[tokio::test]
async fn write_note_write_empty_name_and_contents() {
    let io = TestStorageIo::new();
    let mut storage = make_default_limits_storage("/", io)
        .await.expect("storage creation failed");
    storage.write_note(
        &UsernameString::from_str("write_note").unwrap(),
        &Note {
            id: *WRITE_NOTE_NORMAL_UUID,
            name: None,
            contents: "".into(),
        },
    ).await.expect("write failed");

    let tmp_files = storage.io.get_tmp_files().await;
    assert_eq!(tmp_files.len(), 1);
    let (tmp_file_key, tmp_filename) = &tmp_files[0];
    assert_eq!(*tmp_file_key, make_tmp_path("/write_note", *WRITE_NOTE_NORMAL_UUID).path);
    let events = storage.io.get_events().await;
    assert_eq!(events.len(), 2);
    
    let (path, data) = match events[0] {
        StorageWrite::Write { ref path, ref data } => (path, data),
        _ => panic!("not a write event: {:?}", events[0]),
    };
    assert_eq!(*path, PathBuf::from(tmp_filename));
    assert!(*data == "\n".bytes().collect::<Vec<_>>() || data.is_empty());
}

#[tokio::test]
async fn write_note_write_error() {
    let io = TestStorageIo::new();
    let mut storage = make_default_limits_storage("/", io)
        .await.expect("storage creation failed");
    storage.write_note(
        &UsernameString::from_str("write_note").unwrap(),
        &Note {
            id: *WRITE_NOTE_CANT_WRITE_UUID,
            name: None,
            contents: "".into(),
        },
    ).await.expect_err("should fail");

    let tmp_files = storage.io.get_tmp_files().await;
    assert_eq!(tmp_files.len(), 0);
    let events = storage.io.get_events().await;
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], StorageWrite::Write { .. }));
}

#[tokio::test]
async fn write_note_rename_error() {
    write_note_rename_error_impl().await;
}

#[tokio::test]
async fn write_note_remove_after_renaming_fail_error() {
    write_note_rename_error_impl().await;
}

async fn write_note_rename_error_impl() {
    let io = TestStorageIo::new();
    let mut storage = make_default_limits_storage("/", io)
        .await.expect("storage creation failed");
    storage.write_note(
        &UsernameString::from_str("write_note").unwrap(),
        &Note {
            id: *WRITE_NOTE_CANT_RENAME_CANT_REMOVE_UUID,
            name: None,
            contents: "".into(),
        },
    ).await.expect_err("should fail");

    let tmp_files = storage.io.get_tmp_files().await;
    assert_eq!(tmp_files.len(), 1);
    let events = storage.io.get_events().await;
    assert_eq!(events.len(), 3);
    assert!(matches!(events[0], StorageWrite::Write { .. }));
    assert!(matches!(events[1], StorageWrite::Rename { .. }));
    assert!(matches!(events[2], StorageWrite::Remove { .. }));
}

#[tokio::test]
async fn list_notes_empty() {
    let io = TestStorageIo::new();
    let mut storage = make_default_limits_storage("/", io)
        .await.expect("storage creation failed");
    let dir = storage.list_notes(&UsernameString::from_str("empty_dir").unwrap())
        .await.expect("directory read failed");
    assert_eq!(dir.len(), 0);
}

#[tokio::test]
async fn list_notes_error_listing() {
    let io = TestStorageIo::new();
    let mut storage = make_default_limits_storage("/", io)
        .await.expect("storage creation failed");
    storage.list_notes(&UsernameString::from_str("not_enough_perms_dir").unwrap())
        .await.expect_err("should fail");
}

#[tokio::test]
async fn get_note_details_normal() {
    let note = get_single_note_details_successfully(*READ_NOTE_NORMAL_UUID).await;
    assert_eq!(note.name, Some("normal title".into()));
}

#[tokio::test]
async fn get_note_details_empty_file() {
    let note = get_single_note_details_successfully(*READ_NOTE_EMPTY_UUID).await;
    assert_eq!(note.name, None);
}

#[tokio::test]
async fn get_note_details_empty_name() {
    let note = get_single_note_details_successfully(*READ_NOTE_EMPTY_NAME_UUID).await;
    assert_eq!(note.name, None);
}

#[tokio::test]
async fn get_note_details_empty_contents() {
    let note = get_single_note_details_successfully(*READ_NOTE_EMPTY_CONTENTS_UUID).await;
    assert_eq!(note.name, Some("normal title".into()));
}

#[tokio::test]
async fn get_note_details_cant_open() {
    get_single_note_details_with_error(*READ_NOTE_CANT_OPEN_UUID).await
}

#[tokio::test]
async fn get_note_details_cant_read() {
    get_single_note_details_with_error(*READ_NOTE_CANT_READ_UUID).await
}

#[tokio::test]
async fn get_note_details_invalid_utf8() {
    let note = get_single_note_details_successfully(*READ_NOTE_INVALID_UTF8_UUID).await;
    assert_eq!(note.name, Some(READ_NOTE_INVALID_UTF8_TITLE.into()));
}

#[tokio::test]
async fn get_note_details_file_too_big() {
    todo!("implement the test after the config is done")
}

#[tokio::test]
async fn get_note_details_name_too_big() {
    todo!("implement the test after the config is done")
}

#[tokio::test]
async fn get_note_details_name_not_terminated_with_newline() {
    let note = get_single_note_details_successfully(*READ_NOTE_NO_NEWLINE_UUID).await;
    assert_eq!(note.name, Some("normal title".into()));
}

#[tokio::test]
async fn get_note_details_file_became_too_big_after_metadata_read() {
    todo!("implement the test after the config is done")
}

async fn get_single_note_details_successfully(id: Uuid) -> NoteInfo {
    let io = TestStorageIo::new();
    let mut storage = make_default_limits_storage("/", io)
        .await.expect("storage creation failed");
    let mut res = storage.get_note_details(
        &UsernameString::from_str("read_note").unwrap(),
        vec![
            NoteMetadata {
                id,
                mtime: UtcDateTime::from_unix_timestamp(42).unwrap()
            }
        ],
    ).await.expect("note read failed");
    assert_eq!(res.len(), 1);
    assert!(res[0].is_some());
    let note = res.remove(0).unwrap();
    assert_eq!(note.metadata.id, id);
    assert_eq!(note.metadata.mtime, UtcDateTime::from_unix_timestamp(42).unwrap());
    note
}

async fn get_single_note_details_with_error(id: Uuid) {
    let io = TestStorageIo::new();
    let mut storage = make_default_limits_storage("/", io)
        .await.expect("storage creation failed");
    let res = storage.get_note_details(
        &UsernameString::from_str("read_note").unwrap(),
        vec![
            NoteMetadata {
                id,
                mtime: UtcDateTime::from_unix_timestamp(42).unwrap()
            }
        ],
    ).await.expect("getting note details failed");
    assert_eq!(res.len(), 1);
    assert!(res[0].is_none());
}

#[tokio::test]
async fn get_note_details_multiple() {
    let io = TestStorageIo::new();
    let mut storage = make_default_limits_storage("/", io)
        .await.expect("storage creation failed");
    let ids = [
        *READ_NOTE_NORMAL_UUID,
        *READ_NOTE_EMPTY_UUID,
        *READ_NOTE_EMPTY_NAME_UUID,
        *READ_NOTE_EMPTY_CONTENTS_UUID,
        *READ_NOTE_CANT_OPEN_UUID,
        *READ_NOTE_CANT_READ_UUID,
        *READ_NOTE_INVALID_UTF8_UUID,
        *READ_NOTE_NO_NEWLINE_UUID,
    ];
    let res = storage.get_note_details(
        &UsernameString::from_str("read_note").unwrap(),
        ids.iter()
            .map(|id| NoteMetadata {
                id: *id,
                mtime: UtcDateTime::from_unix_timestamp(42).unwrap(),
            })
            .collect::<Vec<_>>(),
    ).await.expect("getting note details failed");
    assert_eq!(res.len(), ids.len());
    assert!(res[0].is_some());
    assert!(res[1].is_some());
    assert!(res[2].is_some());
    assert!(res[3].is_some());
    assert!(res[4].is_none());
    assert!(res[5].is_none());
    assert!(res[6].is_some());
    assert!(res[7].is_some());
}

#[tokio::test]
async fn delete_note_successfully() {
    let io = TestStorageIo::new();
    let mut storage = make_default_limits_storage("/", io)
        .await.expect("storage creation failed");
    storage.delete_note(
        &UsernameString::from_str("delete_note").unwrap(),
        *DELETE_NOTE_SUCCESS
    ).await.expect("deleting note failed");
}

#[tokio::test]
async fn delete_note_error() {
    let io = TestStorageIo::new();
    let mut storage = make_default_limits_storage("/", io)
        .await.expect("storage creation failed");
    storage.delete_note(
        &UsernameString::from_str("delete_note").unwrap(),
        *DELETE_NOTE_ERROR
    ).await.expect_err("should fail");
}

async fn make_default_limits_storage<S: NoteStorageIo>(
    basedir: &str,
    io: S,
) -> Result<NoteStorageImpl<S>, StorageError> {
    NoteStorageImpl::new_internal(
        &serde_json::from_str::<AppConfig>(
            &format!("{{\"data_directory\": \"{basedir}\"}}")
        ).unwrap(),
        io
    ).await
}
