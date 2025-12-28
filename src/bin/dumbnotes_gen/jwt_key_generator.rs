use std::io;
use std::io::Write;
use std::path::Path;
use josekit::JoseError;
use josekit::jwk::alg::ed::EdCurve;
use josekit::jwk::Jwk;
use thiserror::Error;
use file_write::{get_ids_for_chown, write};
use crate::file_write;

#[derive(Debug, Error)]
pub enum MakeJwtKeyError {
    #[error("failed generating jwt key")]
    Generation(#[from] JoseError),

    #[error("jwt key serialization failed")]
    Serialization(#[from] serde_json::Error),

    #[error("failed writing generated jwt key: {0}")]
    Io(#[from] io::Error),
}

pub fn make_jwt_key(
    jwt_private_key: &Path,
    jwt_public_key: &Path,
    owner_user_group: Option<&str>,
) -> Result<(), MakeJwtKeyError> {
    let private_key = Jwk::generate_ed_key(EdCurve::Ed25519)?;
    let public_key = private_key.to_public_key()?;
    let (uid, gid) = get_ids_for_chown(owner_user_group)?;
    write(
        jwt_private_key,
        serde_json::to_string_pretty(&private_key)? + "\n",
        uid,
        gid,
        Some(0o440),
    )?;
    write(
        jwt_public_key,
        serde_json::to_string_pretty(&public_key)? + "\n",
        None,
        None,
        None,
    )?;
    Ok(())
}
