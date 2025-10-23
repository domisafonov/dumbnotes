use std::str::FromStr;
use josekit::jwt;
use josekit::jwk::Jwk;
use josekit::jws::alg::hmac::{HmacJwsAlgorithm, HmacJwsVerifier};
use time::OffsetDateTime;
use uuid::Uuid;
use dumbnotes::username_string::UsernameString;
use errors::AccessTokenDecoderError;
use crate::access_token::data::AccessTokenData;

pub(super) mod errors;

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
            .transpose()
            .map_err(AccessTokenDecoderError::PayloadParse)?
            .ok_or(AccessTokenDecoderError::missing("session_id"))?;
        let username = payload.subject()
            .map(UsernameString::from_str)
            .transpose()?
            .ok_or(AccessTokenDecoderError::missing("subject"))?;
        let not_before = payload.not_before()
            .map(OffsetDateTime::from)
            .ok_or(AccessTokenDecoderError::missing("not_before"))?;
        let expires_at = payload.expires_at()
            .map(OffsetDateTime::from)
            .ok_or(AccessTokenDecoderError::missing("expires_at"))?;
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
