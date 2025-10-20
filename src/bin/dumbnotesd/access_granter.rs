use std::error::Error;
use std::fmt::{Display, Formatter};
use std::time::SystemTime;
use uuid::Uuid;
use dumbnotes::username_string::UsernameString;
use crate::access_token::{AccessTokenDecoder, AccessTokenGenerator};
use crate::session_storage::SessionStorage;
use crate::user_db::UserDb;

pub struct AccessGranter {
    session_storage: Box<dyn SessionStorage>,
    user_db: Box<dyn UserDb>,
    access_token_generator: AccessTokenGenerator,
    access_token_decoder: AccessTokenDecoder,
}

pub struct LoginResult {
    pub refresh_token: Vec<u8>,
    pub access_token: String,
}

#[derive(Debug)]
pub enum SessionInfo {
    Valid(ValidSession),
    Expired,
}

#[derive(Debug)]
pub struct ValidSession {
    session_id: Uuid,
    username: UsernameString,
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
        let token = auth_header_value.strip_prefix("Bearer:")
            .ok_or(AccessGranterError::HeaderFormatError)?
            .trim_ascii_start();
        let token = self.access_token_decoder.decode_token(token)
            .map_err(|_| AccessGranterError::InvalidToken)?;
        let now = SystemTime::now();
        if token.not_before > now || now <= token.expires_at {
            Ok(SessionInfo::Expired)
        } else {
            Ok(
                SessionInfo::Valid(
                    ValidSession {
                        session_id: token.session_id,
                        username: token.username,
                    }
                )
            )
        }
    }

    pub async fn login_user(
        &self,
        password: &str,
    ) -> Result<LoginResult, AccessGranterError> {
        todo!()
    }

    pub async fn refresh_user_token(
        &self,
        refresh_token: &[u8],
    ) -> Result<LoginResult, AccessGranterError> {
        todo!()
    }

    pub async fn logout_user(
        &self,
        session_id: Uuid,
    ) -> Result<(), AccessGranterError> {
        todo!()
    }
}

#[derive(Debug)]
pub enum AccessGranterError {
    HeaderFormatError,
    InvalidToken,
}

impl Display for AccessGranterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessGranterError::HeaderFormatError => f.write_str("Token format error"),
            AccessGranterError::InvalidToken => f.write_str("Invalid token"),
        }
    }
}
impl Error for AccessGranterError {}
