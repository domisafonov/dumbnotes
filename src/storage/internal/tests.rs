use std::str::FromStr;
use crate::config::UsernameString;
use super::*;

// TODO: remember to test errors being logged

#[tokio::test] // TODO: delete
async fn test() {
    let mut storage = NoteStorage::new("/Users/").await.unwrap();
    storage.get_user_dir(&UsernameString::from_str("abcdef").unwrap());
}

#[tokio::test]
async fn create_storage_ok() {
    todo!()
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