use std::io;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use josekit::JoseError;
use josekit::jwk::alg::ed::EdCurve;
use josekit::jwk::Jwk;
use thiserror::Error;

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
) -> Result<(), MakeJwtKeyError> {
    let private_key = Jwk::generate_ed_key(EdCurve::Ed25519)?;
    let public_key = private_key.to_public_key()?;
    write(
        jwt_private_key,
        serde_json::to_string_pretty(&private_key)? + "\n",
        Some(0o600),
    )?;
    write(
        jwt_public_key,
        serde_json::to_string_pretty(&public_key)? + "\n",
        None,
    )?;
    Ok(())
}

fn write(
    path: &Path,
    contents: impl AsRef<str>,
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
    file.write_all(contents.as_ref().as_bytes())?;
    Ok(())
}
