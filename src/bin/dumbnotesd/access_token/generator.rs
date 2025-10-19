use std::fmt::Display;
use std::fs;
use std::ops::Add;
use std::path::Path;
use std::time::{Duration, SystemTime};
use josekit::jwt;
use josekit::jwk::Jwk;
use josekit::jws::alg::hmac::{HmacJwsAlgorithm, HmacJwsSigner};
use josekit::jws::JwsHeader;
use josekit::jwt::JwtPayload;
use errors::AccessTokenGeneratorError;
use crate::session_storage::Session;

pub(super) mod errors;

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
