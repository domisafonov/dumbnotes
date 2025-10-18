use std::fmt::{Display, Formatter};
use std::{fs, io};
use std::path::Path;
use std::str::FromStr;
use josekit::{jwt, JoseError};
use josekit::jwk::Jwk;
use josekit::jws::alg::hmac::{HmacJwsAlgorithm, HmacJwsVerifier};
use time::OffsetDateTime;
use uuid::Uuid;
use dumbnotes::username_string::{UsernameParseError, UsernameString};
use crate::access_token::data::AccessTokenData;

pub struct AccessTokenDecoder {
    verifier: HmacJwsVerifier,
}

impl AccessTokenDecoder {
    pub fn from_jwk(jwk: &Jwk) -> Result<Self, AccessTokenDecoderError> {
        Ok(
            AccessTokenDecoder {
                verifier: HmacJwsAlgorithm::Hs512.verifier_from_jwk(jwk)?,
            }
        )
    }

    pub fn from_file(
        path: impl AsRef<Path>,
    ) -> Result<Self, AccessTokenDecoderError> {
        Self::from_jwk(&Jwk::from_bytes(fs::read(path)?)?)
    }

    // TODO: setup the context
    pub fn decode_token(
        &self,
        token: impl AsRef<[u8]>,
    ) -> Result<AccessTokenData, AccessTokenDecoderError> {
        let (payload, header) = jwt::decode_with_verifier(
            token.as_ref(),
            &self.verifier,
        )?;
        let session_id = payload.claim("session_id")
            .map(|v| serde_json::from_value::<Uuid>(v.clone()))
            .transpose()?
            .map(Ok)
            .unwrap_or(Err(AccessTokenDecoderError::PayloadError))?;
        let username = payload.subject()
            .map(|s| UsernameString::from_str(s))
            .transpose()?
            .map(Ok)
            .unwrap_or(Err(AccessTokenDecoderError::PayloadError))?;
        let not_before = payload.not_before()
            .map(OffsetDateTime::from)
            .map(Ok)
            .unwrap_or(Err(AccessTokenDecoderError::PayloadError))?;
        let expires_at = payload.expires_at()
            .map(OffsetDateTime::from)
            .map(Ok)
            .unwrap_or(Err(AccessTokenDecoderError::PayloadError))?;
        Ok(
            AccessTokenData {
                session_id,
                username,
                not_before,
                expires_at,
            }
        )
    }
}

#[derive(Debug)]
pub enum AccessTokenDecoderError {
    CryptoError(JoseError),
    IoError(io::Error),
    PayloadError,
}

impl Display for AccessTokenDecoderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessTokenDecoderError::CryptoError(e) =>
                f.write_fmt(format_args!("{e}")),
            AccessTokenDecoderError::IoError(e) =>
                f.write_fmt(format_args!("{e}")),
            AccessTokenDecoderError::PayloadError =>
                f.write_str("Error decoding payload"),
        }
    }
}
impl std::error::Error for AccessTokenDecoderError {}

impl From<JoseError> for AccessTokenDecoderError {
    fn from(e: JoseError) -> AccessTokenDecoderError {
        AccessTokenDecoderError::CryptoError(e)
    }
}

impl From<io::Error> for AccessTokenDecoderError {
    fn from(e: io::Error) -> AccessTokenDecoderError {
        AccessTokenDecoderError::IoError(e)
    }
}

impl From<serde_json::error::Error> for AccessTokenDecoderError {
    fn from(_: serde_json::error::Error) -> AccessTokenDecoderError {
        AccessTokenDecoderError::PayloadError
    }
}

impl From<UsernameParseError> for AccessTokenDecoderError {
    fn from(_: UsernameParseError) -> AccessTokenDecoderError {
        AccessTokenDecoderError::PayloadError
    }
}
