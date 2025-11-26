use std::{fs, io};
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
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
    fs::write(
        jwt_private_key,
        serde_json::to_string_pretty(&private_key)? + "\n",
    )?;
    fs::set_permissions(
        jwt_private_key,
        Permissions::from_mode(0o700),
    )?;
    fs::write(
        jwt_public_key,
        serde_json::to_string_pretty(&public_key)? + "\n",
    )?;
    Ok(())
}
