use std::fmt::{Display, Formatter};
use std::{fs, io};
use std::ops::Add;
use std::path::Path;
use std::time::{Duration, SystemTime};
use josekit::{jwt, JoseError};
use josekit::jwk::Jwk;
use josekit::jws::alg::hmac::{HmacJwsAlgorithm, HmacJwsSigner};
use josekit::jws::JwsHeader;
use josekit::jwt::JwtPayload;
use crate::session_storage::Session;

pub struct AccessTokenGenerator {
    signer: HmacJwsSigner,
}

impl AccessTokenGenerator {
    pub fn from_jwk(key: &Jwk) -> Result<Self, AccessTokenGeneratorError> {
        Ok(
            AccessTokenGenerator {
                signer: HmacJwsAlgorithm::Hs512.signer_from_jwk(key)?,
            }
        )
    }

    pub fn from_file(
        path: impl AsRef<Path>,
    ) -> Result<Self, AccessTokenGeneratorError> {
        Self::from_jwk(&Jwk::from_bytes(fs::read(path)?)?)
    }

    // TODO: setup the context
    pub fn generate_token(
        &self,
        session: &Session,
        now: SystemTime,
    ) -> Result<String, AccessTokenGeneratorError> {
        let mut payload = JwtPayload::new();
        payload.set_subject(session.username.to_string());
        payload.set_claim("session_id", Some(serde_json::to_value(session.session_id)?))?;
        payload.set_not_before(&now);
        payload.set_expires_at(&now.add(Duration::new(15 * 60, 0)));

        Ok(
            jwt::encode_with_signer(
                &payload,
                &JwsHeader::new(),
                &self.signer,
            )?
        )
    }
}

#[derive(Debug)]
pub enum AccessTokenGeneratorError {
    CryptoError(JoseError),
    IoError(io::Error),
    SerializationError,
}

impl Display for AccessTokenGeneratorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessTokenGeneratorError::CryptoError(e) =>
                f.write_fmt(format_args!("{e}")),
            AccessTokenGeneratorError::IoError(e) =>
                f.write_fmt(format_args!("{e}")),
            AccessTokenGeneratorError::SerializationError =>
                f.write_str("Error serializing JWT claims"),
        }
    }
}
impl std::error::Error for AccessTokenGeneratorError {}

impl From<JoseError> for AccessTokenGeneratorError {
    fn from(e: JoseError) -> AccessTokenGeneratorError {
        AccessTokenGeneratorError::CryptoError(e)
    }
}

impl From<io::Error> for AccessTokenGeneratorError {
    fn from(e: io::Error) -> AccessTokenGeneratorError {
        AccessTokenGeneratorError::IoError(e)
    }
}

impl From<serde_json::Error> for AccessTokenGeneratorError {
    fn from(_: serde_json::Error) -> AccessTokenGeneratorError {
        AccessTokenGeneratorError::SerializationError
    }
}
