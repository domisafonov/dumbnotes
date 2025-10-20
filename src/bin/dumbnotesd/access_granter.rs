use std::error::Error;
use std::fmt::{Display, Formatter};
use std::time::SystemTime;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;
use dumbnotes::username_string::{UsernameStr, UsernameString};
use crate::access_token::{AccessTokenDecoder, AccessTokenGenerator, AccessTokenGeneratorError};
use crate::session_storage::{SessionStorage, SessionStorageError};
use crate::user_db::{UserDb, UserDbError};

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
    Valid(KnownSession),
    Expired(KnownSession),
}

#[derive(Debug)]
pub struct KnownSession {
    pub session_id: Uuid,
    pub username: UsernameString,
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
        let token = auth_header_value.strip_prefix("Bearer")
            .ok_or(AccessGranterError::HeaderFormatError)?
            .trim_ascii_start();
        let token = self.access_token_decoder.decode_token(token)
            .map_err(|_| AccessGranterError::InvalidToken)?;
        let known_session = KnownSession {
            session_id: token.session_id,
            username: token.username,
        };
        let now = SystemTime::now();
        Ok(
            if token.not_before > now || now >= token.expires_at {
                SessionInfo::Expired(known_session)
            } else {
                SessionInfo::Valid(known_session)
            }
        )
    }

    pub async fn login_user(
        &self,
        username: &UsernameStr,
        password: &str,
    ) -> Result<LoginResult, AccessGranterError> {
        if self.user_db.check_user_credentials(username, password).await? {
            let now = OffsetDateTime::now_utc();
            let session = self.session_storage
                .create_session(
                    username,
                    now,
                    now + Duration::minutes(15),
                )
                .await?;
            let access_token = self.access_token_generator
                .generate_token(&session, now.into())?;
            Ok(
                LoginResult {
                    refresh_token: session.refresh_token,
                    access_token,
                }
            )
        } else {
            Err(AccessGranterError::InvalidCredentials)
        }
    }

    pub async fn refresh_user_token(
        &self,
        username: &UsernameStr,
        refresh_token: &[u8],
    ) -> Result<LoginResult, AccessGranterError> {
        let session = self.session_storage
            .get_session_by_token(refresh_token)
            .await?;
        if let Some(session) = session {
            if session.username.as_str() != username {
                return Err(AccessGranterError::InvalidCredentials);
            }
        }
        let now = OffsetDateTime::now_utc();
        let session = self.session_storage
            .refresh_session(
                refresh_token,
                now + Duration::minutes(15)
            )
            .await?;
        let access_token = self.access_token_generator
            .generate_token(&session, now.into())?;
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
        self.session_storage.delete_session(session_id).await?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum AccessGranterError {
    HeaderFormatError,
    InvalidToken,
    InvalidCredentials,
    SessionStorageError(SessionStorageError),
    UserDbError(UserDbError),
    AccessTokenGeneratorError(AccessTokenGeneratorError),
}

impl Display for AccessGranterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessGranterError::HeaderFormatError =>
                f.write_str("Token format error"),
            AccessGranterError::InvalidToken => f.write_str("Invalid token"),
            AccessGranterError::InvalidCredentials =>
                f.write_str("Invalid credentials"),
            AccessGranterError::SessionStorageError(e) =>
                f.write_fmt(format_args!("{e}")),
            AccessGranterError::UserDbError(e) =>
                f.write_fmt(format_args!("{e}")),
            AccessGranterError::AccessTokenGeneratorError(e) =>
                f.write_fmt(format_args!("{e}")),
        }
    }
}
impl Error for AccessGranterError {}

impl From<SessionStorageError> for AccessGranterError {
    fn from(e: SessionStorageError) -> Self {
        match e {
            SessionStorageError::SessionNotFound => AccessGranterError::InvalidCredentials,
            _ => AccessGranterError::SessionStorageError(e),
        }
    }
}

impl From<UserDbError> for AccessGranterError {
    fn from(e: UserDbError) -> Self {
        AccessGranterError::UserDbError(e)
    }
}

impl From<AccessTokenGeneratorError> for AccessGranterError {
    fn from(e: AccessTokenGeneratorError) -> Self {
        AccessGranterError::AccessTokenGeneratorError(e)
    }
}
