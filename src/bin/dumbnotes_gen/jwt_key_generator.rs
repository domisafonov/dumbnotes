use std::io;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use josekit::JoseError;
use josekit::jwk::alg::ed::EdCurve;
use josekit::jwk::Jwk;
use libc::{gid_t, uid_t};
use thiserror::Error;
use dumbnotes::nix::{get_ids, ChownExt};
use dumbnotes::sandbox::user_group::get_user_and_group;
#[derive(Debug, Error)]
pub enum MakeJwtKeyError {
    #[error("failed generating jwt key")]
    Generation(#[from] JoseError),

    #[error("jwt key serialization failed")]
    Serialization(#[from] serde_json::Error),

    #[error("failed writing generated jwt key")]
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
        Some(0o640),
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

fn get_ids_for_chown(
    owner_user_group: Option<&str>,
) -> Result<(Option<uid_t>, Option<gid_t>), MakeJwtKeyError> {
    let owner_user_group = match owner_user_group {
        Some(value) => value,
        None => return Ok((None, None)),
    };
    let (expected_uid, expected_gid) = get_user_and_group(owner_user_group)?;
    let (uid, gid) = get_ids();
    Ok((
        Some(expected_uid).filter(|e| *e != uid),
        Some(expected_gid).filter(|e| *e != gid),
    ))
}

fn write(
    path: &Path,
    contents: impl AsRef<str>,
    uid: Option<uid_t>,
    gid: Option<gid_t>,
    mode: Option<u32>,
) -> Result<(), io::Error> {
    let mut options = OpenOptions::new();
    if let Some(mode) = mode {
        options.mode(mode);
    }
    let mut file = options
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    file.chown(uid, gid)?;
    file.write_all(contents.as_ref().as_bytes())?;
    Ok(())
}
