use std::time::SystemTime;
use josekit::jwt;
use josekit::jwk::Jwk;
use josekit::jws::alg::hmac::{HmacJwsAlgorithm, HmacJwsSigner};
use josekit::jws::JwsHeader;
use josekit::jwt::JwtPayload;
use log::{debug, log_enabled};
use errors::AccessTokenGeneratorError;
use crate::access_token::data::SESSION_ID_CLAIM_NAME;
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

    pub fn generate_token(
        &self,
        session: &Session,
        now: SystemTime,
    ) -> Result<String, AccessTokenGeneratorError> {
        let mut payload = JwtPayload::new();
        let subject = session.username.to_string();
        let session_id = serde_json::to_value(session.session_id)
            .map_err(AccessTokenGeneratorError::SessionIdSerialization)?;
        let session_id_str = if log_enabled!(log::Level::Debug) {
            &session_id.to_string() 
        } else {
            ""
        };
        let not_before = &now;
        let expires_at = session.expires_at.into();
        payload.set_subject(&subject);
        payload.set_claim(
            SESSION_ID_CLAIM_NAME,
            Some(session_id)
        )?;
        payload.set_not_before(not_before);
        payload.set_expires_at(&expires_at);

        let token = jwt::encode_with_signer(
            &payload,
            &JwsHeader::new(),
            &self.signer,
        )?;
        debug!(
            "access token {token} generated with subject {subject}, \
                session id {session_id_str}, \
                not_before {not_before:?}, \
                expires_at {expires_at:?}"
        );
        Ok(token)
    }
}
