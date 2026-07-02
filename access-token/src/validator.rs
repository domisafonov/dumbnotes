use std::time::SystemTime;

use log::{trace, warn};
use thiserror::Error;

use crate::{AccessTokenDecoder, AccessTokenDecoderError, data::AccessTokenData};

pub struct AccessTokenValidator {
    access_token_decoder: AccessTokenDecoder,
}

impl AccessTokenValidator {
    pub fn new(
        access_token_decoder: AccessTokenDecoder,
    ) -> Self {
        AccessTokenValidator {
            access_token_decoder,
        }
    }

    pub fn check_access_token(
        &self,
        access_token: impl AsRef<[u8]>,
    ) -> Result<AccessTokenData, AccessTokenValidatorError> {
        let access_token = self.access_token_decoder.decode_token(access_token)
            .map_err(|e| {
                warn!("failed to decode token: {}", e);
                AccessTokenValidatorError::InvalidToken(e)
            })?;
        self.check_decoded_access_token(&access_token)?;
        Ok(access_token)
    }

    pub fn check_decoded_access_token(
        &self,
        access_token: &AccessTokenData,
    ) -> Result<(), AccessTokenValidatorError> {
        let now = SystemTime::now();
        if access_token.not_before > now || now >= access_token.expires_at {
            trace!(
                "expired valid token for user \"{}\"",
                access_token.username,
            );
            Err(AccessTokenValidatorError::ExpiredToken(access_token.clone()))
        } else {
            trace!("valid token for user \"{}\"", access_token.username);
            Ok(())
        }
    }
}

#[derive(Debug, Error)]
pub enum AccessTokenValidatorError {
    #[error("invalid token: {0}")]
    InvalidToken(AccessTokenDecoderError),

    #[error("expired token")]
    ExpiredToken(AccessTokenData),
}
