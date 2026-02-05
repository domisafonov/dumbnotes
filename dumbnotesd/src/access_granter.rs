use dumbnotes::access_token::AccessTokenDecoder;
use data::UsernameStr;
use std::time::SystemTime;
use async_trait::async_trait;
use log::{debug, trace, warn};
use tokio::net::UnixStream;
use uuid::Uuid;
use dumbnotes::ipc::auth::caller::{Caller, ProductionCaller};
use auth_ipc_data::model::login::{LoginRequest, LoginResponse};
use auth_ipc_data::model::logout::{LogoutRequest, LogoutResponse};
use auth_ipc_data::model::refresh_token::{RefreshTokenRequest, RefreshTokenResponse};
use auth_ipc_data::bindings::{LoginError, LogoutError};

mod errors;
mod model;

pub use errors::AccessGranterError;
pub use model::{KnownSession, LoginResult, SessionInfo};

#[async_trait]
pub trait AccessGranter: Send + Sync + 'static {
    async fn check_user_access(
        &self,
        auth_header_value: &str,
    ) -> Result<SessionInfo, AccessGranterError>;

    async fn login_user(
        &self,
        username: &UsernameStr,
        password: &str,
    ) -> Result<LoginResult, AccessGranterError>;

    async fn refresh_user_token(
        &self,
        username: &UsernameStr,
        refresh_token: &[u8],
    ) -> Result<LoginResult, AccessGranterError>;

    async fn logout_user(
        &self,
        session_id: Uuid,
    ) -> Result<(), AccessGranterError>;
}

pub type ProductionAccessGranter = AccessGranterImpl<ProductionCaller>;

pub struct AccessGranterImpl<C: Caller> {
    access_token_decoder: AccessTokenDecoder,
    caller: C,
}

impl ProductionAccessGranter {
    pub async fn new(
        access_token_decoder: AccessTokenDecoder,
        auth_socket: UnixStream,
    ) -> Self {
        AccessGranterImpl {
            access_token_decoder,
            caller: ProductionCaller::new(auth_socket).await,
        }
    }
}

#[async_trait]
impl<C: Caller> AccessGranter for AccessGranterImpl<C> {
    async fn check_user_access(
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

    async fn login_user(
        &self,
        username: &UsernameStr,
        password: &str,
    ) -> Result<LoginResult, AccessGranterError> {
        debug!("logging user \"{username}\" in");
        let response: LoginResponse = self.caller
            .execute(
                auth_ipc_data::bindings::command::Command::Login(
                    LoginRequest {
                        username: username.to_owned(),
                        password: password.to_owned(),
                    }.into()
                )
            )
            .await?
            .try_into()?;
        match response.0 {
            Ok(successful_login) => Ok(LoginResult {
                refresh_token: successful_login.refresh_token,
                access_token: successful_login.access_token,
            }),
            Err(e) => Err(
                match e {
                    LoginError::LoginInvalidCredentials => AccessGranterError::InvalidCredentials,
                    LoginError::LoginInternalError => AccessGranterError::AuthDaemonInternalError,
                }
            )
        }
    }

    async fn refresh_user_token(
        &self,
        username: &UsernameStr,
        refresh_token: &[u8],
    ) -> Result<LoginResult, AccessGranterError> {
        debug!("refreshing access token for user \"{username}\"");
        let response: RefreshTokenResponse = self.caller
            .execute(
                auth_ipc_data::bindings::command::Command::RefreshToken(
                    RefreshTokenRequest {
                        username: username.to_owned(),
                        refresh_token: refresh_token.to_owned(),
                    }.into()
                )
            )
            .await?
            .try_into()?;
        match response.0 {
            Ok(successful_login) => Ok(LoginResult {
                refresh_token: successful_login.refresh_token,
                access_token: successful_login.access_token,
            }),
            Err(e) => Err(
                match e {
                    LoginError::LoginInvalidCredentials => AccessGranterError::InvalidCredentials,
                    LoginError::LoginInternalError => AccessGranterError::AuthDaemonInternalError,
                }
            )
        }
    }

    async fn logout_user(
        &self,
        session_id: Uuid,
    ) -> Result<(), AccessGranterError> {
        debug!("deleting session {session_id}");
        let response: LogoutResponse = self.caller
            .execute(
                auth_ipc_data::bindings::command::Command::Logout(
                    LogoutRequest {
                        session_id,
                    }.into()
                )
            )
            .await?
            .try_into()?;
        match response.0 {
            Some(error) => Err(
                match error {
                    LogoutError::LogoutInternalError => AccessGranterError::AuthDaemonInternalError,
                }
            ),
            None => Ok(())
        }
    }
}
