use access_token_data::{SESSION_ID_JWT_CLAIM_NAME, SESSION_KIND_API, SESSION_KIND_JWT_CLAIM_NAME, SESSION_KIND_WEB};
use data::{SessionKind, UsernameStr};
use josekit::jwk::Jwk;
use josekit::jws::{EdDSA, JwsHeader};
use josekit::{JoseError, jwt};
use josekit::jwt::JwtPayload;
use log::{debug, log_enabled};
use serde_json::Value;
use thiserror::Error;
use std::io;
use std::time::SystemTime;
use josekit::jws::alg::eddsa::EddsaJwsSigner;
use uuid::Uuid;

pub struct AccessTokenGenerator {
    signer: EddsaJwsSigner,
}

impl AccessTokenGenerator {
    pub fn from_jwk(key: &Jwk) -> Result<Self, AccessTokenGeneratorError> {
        Ok(
            AccessTokenGenerator {
                signer: EdDSA.signer_from_jwk(key)?,
            }
        )
    }

    pub fn generate_token(
        &self,
        session_id: Uuid,
        username: &UsernameStr,
        not_before: &SystemTime,
        expires_at: &SystemTime,
        session_kind: SessionKind,
    ) -> Result<String, AccessTokenGeneratorError> {
        let mut payload = JwtPayload::new();
        let subject = username.to_string();
        let session_id = serde_json::to_value(session_id)
            .map_err(AccessTokenGeneratorError::SessionIdSerialization)?;
        let session_id_str = if log_enabled!(log::Level::Debug) {
            &session_id.to_string() 
        } else {
            ""
        };
        payload.set_subject(&subject);
        payload.set_claim(
            SESSION_ID_JWT_CLAIM_NAME,
            Some(session_id),
        )?;
        payload.set_claim(
            SESSION_KIND_JWT_CLAIM_NAME,
            Some(
                match session_kind {
                    SessionKind::Api => Value::from(SESSION_KIND_API),
                    SessionKind::Web => Value::from(SESSION_KIND_WEB),
                }
            ),
        )?;
        payload.set_not_before(not_before);
        payload.set_expires_at(expires_at);

        let token = jwt::encode_with_signer(
            &payload,
            &JwsHeader::new(),
            &self.signer,
        )?;
        debug!(
            "access token generated with subject {subject}, \
                session id {session_id_str}, \
                not_before {not_before:?}, \
                expires_at {expires_at:?}"
        );
        Ok(token)
    }
}

#[derive(Debug, Error)]
pub enum AccessTokenGeneratorError {
    #[error("cryptographic operation failed: {0}")]
    Crypto(#[from] JoseError),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("access token serialization failed: {0}")]
    SessionIdSerialization(serde_json::Error),
}
