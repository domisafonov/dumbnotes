use dumbnotes::access_token::AccessTokenDecoder;
use dumbnotes::username_string::UsernameStr;
use std::time::SystemTime;
use log::{debug, trace, warn};
use uuid::Uuid;

mod errors;
mod model;

pub use errors::AccessGranterError;
pub use model::{KnownSession, LoginResult, SessionInfo};

pub struct AccessGranter {
    access_token_decoder: AccessTokenDecoder,
}

impl AccessGranter {
    pub fn new(
        access_token_decoder: AccessTokenDecoder,
    ) -> Self {
        AccessGranter {
            access_token_decoder,
        }
    }

    pub async fn check_user_access(
        &self,
        auth_header_value: &str,
    ) -> Result<SessionInfo, AccessGranterError> {
        trace!("authenticating user by header {auth_header_value}");
        let token = auth_header_value.strip_prefix("Bearer ")
            .ok_or(AccessGranterError::HeaderFormatError)?;
        let token = self.access_token_decoder.decode_token(token)
            .map_err(|e| {
                warn!("failed to decode token: {}", e);
                AccessGranterError::InvalidToken
            })?;
        let known_session = KnownSession {
            session_id: token.session_id,
            username: token.username,
        };
        let now = SystemTime::now();
        Ok(
            if token.not_before > now || now >= token.expires_at {
                trace!(
                    "expired valid token for user \"{}\"",
                    known_session.username,
                );
                SessionInfo::Expired(known_session)
            } else {
                trace!("valid token for user \"{}\"", known_session.username);
                SessionInfo::Valid(known_session)
            }
        )
    }

    pub async fn login_user(
        &self,
        username: &UsernameStr,
        password: &str,
    ) -> Result<LoginResult, AccessGranterError> {
        debug!("logging user \"{username}\" in");
        todo!()
    }

    pub async fn refresh_user_token(
        &self,
        username: &UsernameStr,
        refresh_token: &[u8],
    ) -> Result<LoginResult, AccessGranterError> {
        debug!("refreshing access token for user \"{username}\"");
        todo!()
    }

    pub async fn logout_user(
        &self,
        session_id: Uuid,
    ) -> Result<(), AccessGranterError> {
        debug!("deleting session {session_id}");
        todo!()
    }
}
