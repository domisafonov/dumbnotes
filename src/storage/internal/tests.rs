use std::str::FromStr;
use crate::config::UsernameString;
use super::*;

#[tokio::test]
async fn test() {
    let mut storage = NoteStorage::new("/Users/").await.unwrap();
    storage.get_user_dir(&UsernameString::from_str("abcdef").unwrap());
}
