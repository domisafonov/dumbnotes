use dumbnotes::access_token::{AccessTokenDecoder, AccessTokenGenerator};
use dumbnotes::session_storage::SessionStorage;
use crate::user_db::UserDb;
use dumbnotes::username_string::UsernameStr;
use std::time::SystemTime;
use log::{debug, info, trace, warn};
use time::OffsetDateTime;
use uuid::Uuid;

mod errors;
mod model;

pub use errors::AccessGranterError;
pub use model::{KnownSession, LoginResult, SessionInfo};
use crate::app_constants::ACCESS_TOKEN_VALIDITY_TIME;

pub struct AccessGranter {
    session_storage: Box<dyn SessionStorage>,
    user_db: Box<dyn UserDb>,
    access_token_generator: AccessTokenGenerator,
    access_token_decoder: AccessTokenDecoder,
}

impl AccessGranter {
    pub fn new(
        session_storage: Box<dyn SessionStorage>,
        user_db: Box<dyn UserDb>,
        access_token_generator: AccessTokenGenerator,
        access_token_decoder: AccessTokenDecoder
    ) -> Self {
        AccessGranter {
            session_storage,
            user_db,
            access_token_generator,
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
        if self.user_db.check_user_credentials(username, password).await? {
            let now = OffsetDateTime::now_utc();
            let session = self.session_storage
                .create_session(
                    username,
                    now,
                    now + ACCESS_TOKEN_VALIDITY_TIME,
                )
                .await?;
            let access_token = self.access_token_generator
                .generate_token(
                    session.session_id,
                    &session.username,
                    &now.into(),
                    &session.expires_at.into(),
                )?;
            info!(
                "logged user \"{username}\" in with session \"{}\"",
                session.session_id,
            );
            Ok(
                LoginResult {
                    refresh_token: session.refresh_token,
                    access_token,
                }
            )
        } else {
            warn!("invalid credentials for user \"{}\"", username);
            Err(AccessGranterError::InvalidCredentials)
        }
    }

    pub async fn refresh_user_token(
        &self,
        username: &UsernameStr,
        refresh_token: &[u8],
    ) -> Result<LoginResult, AccessGranterError> {
        debug!("refreshing access token for user \"{username}\"");
        let session = self.session_storage
            .get_session_by_token(refresh_token)
            .await?;
        if let Some(session) = session
            && session.username.as_str() != username
        {
            warn!(
                "attempt to refresh access token for nonexisting \
                    or mismatched user \"{username}\""
            );
            return Err(AccessGranterError::InvalidCredentials);
        }
        let now = OffsetDateTime::now_utc();
        let session = self.session_storage
            .refresh_session(
                refresh_token,
                now + ACCESS_TOKEN_VALIDITY_TIME,
            )
            .await?;
        info!(
            "refreshed session {} for user \"{username}\"",
            session.session_id,
        );
        let access_token = self.access_token_generator
            .generate_token(
                session.session_id,
                &session.username,
                &now.into(),
                &session.expires_at.into(),
            )?;
        Ok(
            LoginResult {
                refresh_token: session.refresh_token,
                access_token,
            }
        )
    }

    pub async fn logout_user(
        &self,
        session_id: Uuid,
    ) -> Result<(), AccessGranterError> {
        debug!("deleting session {session_id}");
        let did_exist = self.session_storage
            .delete_session(session_id)
            .await?;
        if did_exist {
            info!("session {session_id} deleted");
        } else {
            warn!("attempting to delete nonexistent session {session_id}");
        }
        Ok(())
    }
}
