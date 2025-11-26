use crate::access_token::data::{AccessTokenData, SESSION_ID_CLAIM_NAME};
use crate::username_string::UsernameString;
use errors::AccessTokenDecoderError;
use josekit::jwk::Jwk;
use josekit::jwt;
use log::info;
use std::str::FromStr;
use josekit::jws::alg::eddsa::EddsaJwsVerifier;
use josekit::jws::EdDSA;
use time::OffsetDateTime;
use uuid::Uuid;

pub mod errors;

pub struct AccessTokenDecoder {
    verifier: EddsaJwsVerifier,
}

impl AccessTokenDecoder {
    pub fn from_jwk(jwk: &Jwk) -> Result<Self, AccessTokenDecoderError> {
        Ok(
            AccessTokenDecoder {
                verifier: EdDSA.verifier_from_jwk(jwk)?,
            }
        )
    }

    /// Decode the access token.
    ///
    /// # Arguments
    /// * [token] — token data to be decoded.
    ///
    /// # Errors
    /// All possible error values signify incorrect [token] data.
    pub fn decode_token(
        &self,
        token: impl AsRef<[u8]>,
    ) -> Result<AccessTokenData, AccessTokenDecoderError> {
        let token = token.as_ref();
        let (payload, _) = jwt::decode_with_verifier(
            token,
            &self.verifier,
        )?;
        let session_id = payload.claim(SESSION_ID_CLAIM_NAME)
            .map(|v| serde_json::from_value::<Uuid>(v.clone()))
            .transpose()
            .map_err(|e| {
                info!(
                    "invalid session_id in access token {}: {e}",
                    String::from_utf8_lossy(token),
                );
                AccessTokenDecoderError::PayloadParse(e)
            })?
            .ok_or_else(|| missing_field(token, SESSION_ID_CLAIM_NAME))?;
        let username = payload.subject()
            .map(UsernameString::from_str)
            .transpose()?
            .ok_or_else(|| missing_field(token, "subject"))?;
        let not_before = payload.not_before()
            .map(OffsetDateTime::from)
            .ok_or_else(|| missing_field(token, "not_before"))?;
        let expires_at = payload.expires_at()
            .map(OffsetDateTime::from)
            .ok_or_else(|| missing_field(token, "expires_at"))?;
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

fn missing_field(token: &[u8], part: &'static str) -> AccessTokenDecoderError {
    info!(
        "missing field {part} in access token {}",
        String::from_utf8_lossy(token),
    );
    AccessTokenDecoderError::PayloadMissing { part }
}
