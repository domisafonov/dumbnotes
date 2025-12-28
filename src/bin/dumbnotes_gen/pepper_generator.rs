use std::io;
use std::path::Path;
use base64ct::{Base64, Encoding};
use rand::RngCore;
use thiserror::Error;
use dumbnotes::bin_constants::PEPPER_LENGTH;
use file_write::{get_ids_for_chown, write};
use crate::file_write;

#[derive(Debug, Error)]
pub enum MakePepperError {
    #[error("failed writing generated pepper: {0}")]
    Io(#[from] io::Error),
}

pub fn make_pepper(
    pepper_path: &Path,
    owner_user_group: Option<&str>,
) -> Result<(), MakePepperError> {
    let mut pepper = [0u8; PEPPER_LENGTH];
    rand::rng().fill_bytes(&mut pepper);
    let (uid, gid) = get_ids_for_chown(owner_user_group)?;
    write(
        pepper_path,
        Base64::encode_string(&pepper) + "\n",
        uid,
        gid,
        Some(0o440),
    )?;
    Ok(())
}
