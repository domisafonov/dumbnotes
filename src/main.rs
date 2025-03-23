use std::error::Error;
use std::str::FromStr;
use uuid::Uuid;

use dumbnotes::config::UsernameString;
use dumbnotes::storage::NoteStorage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut storage = NoteStorage::new("/Users/").await?;
    storage.read_note(&UsernameString::from_str("abc").unwrap(), Uuid::new_v4()).await?;
    println!("Hello, world!");
    Ok(())
}
