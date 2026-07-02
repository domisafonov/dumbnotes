use access_token::{AccessTokenValidator, AccessTokenValidatorError};
use auth_ipc_data::model::successful_login::SuccessfulLogin;
use data::{SessionKind, UsernameStr};
use dumbnotes::bin_constants::IPC_MESSAGE_MAX_SIZE;
use dumbnotes::gen_proto_ipc_wrappers;
use dumbnotes::ipc::data::IpcOutput;
use tokio::sync::oneshot;
use std::marker::PhantomData;
use async_trait::async_trait;
use log::{debug, error, trace};
use tokio::net::UnixStream;
use dumbnotes::ipc::caller::{Caller, CallerImpl};
use auth_ipc_data::model::login::{LoginRequest, LoginResponse};
use auth_ipc_data::model::logout::{LogoutRequest, LogoutResponse};
use auth_ipc_data::model::refresh_token::{RefreshTokenRequest, RefreshTokenResponse};
use auth_ipc_data::bindings::{self, LoginError, LogoutError};

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
        access_token: &str,
    ) -> Result<(), AccessGranterError>;
}

pub struct AccessGranterImpl<
    Command: Send + Sync + 'static,
    CommandContainer: prost::Message + 'static,
    CommandWrapper: IpcOutput<Command, CommandContainer>,
    Response: Send + Sync + 'static,
    C: Caller<Command, CommandContainer, CommandWrapper, Response>,
> {
    access_token_validator: AccessTokenValidator,
    caller: C,
    _phantom: PhantomData<(Command, CommandContainer, CommandWrapper, Response)>,
}

type ProductionCaller = CallerImpl<
    bindings::response::Response,
    bindings::Response,
    Response,
>;

pub type ProductionAccessGranter = AccessGranterImpl<
    bindings::command::Command,
    bindings::Command,
    Command,
    bindings::response::Response,
    ProductionCaller,
>;

gen_proto_ipc_wrappers!(
    bindings::Response[response] | bindings::response::Response => pub Response,
    bindings::Command[command] | bindings::command::Command => pub Command,
);

impl ProductionAccessGranter {
    pub async fn new(
        access_token_validator: AccessTokenValidator,
        auth_socket: UnixStream,
    ) -> (Self, oneshot::Receiver<()>) {
        let (caller, shutdown_notice) = ProductionCaller
            ::new(auth_socket, IPC_MESSAGE_MAX_SIZE)
            .await;
        (
            AccessGranterImpl {
                access_token_validator,
                caller,
                _phantom: Default::default(),
            },
            shutdown_notice,
        )
    }
}

#[async_trait]
impl<
    C: Caller<
        bindings::command::Command,
        bindings::Command,
        Command,
        bindings::response::Response
    >,
> AccessGranter for AccessGranterImpl<bindings::command::Command, bindings::Command, Command, bindings::response::Response, C> {
    async fn check_user_access(
        &self,
        auth_header_value: &str,
    ) -> Result<SessionInfo, AccessGranterError> {
        trace!("authenticating user by header {auth_header_value}");
        let token = auth_header_value.strip_prefix("Bearer ")
            .ok_or(AccessGranterError::HeaderFormatError)?;
        if token.contains(|c: char| c.is_ascii_whitespace()) {
            return Err(AccessGranterError::HeaderFormatError)
        }

        match self.access_token_validator.check_access_token(token) {
            Ok(parsed_token) => Ok(
                SessionInfo::Valid(
                    KnownSession {
                        raw_token: token.to_owned(),
                        session_id: parsed_token.session_id,
                        username: parsed_token.username,
                    }
                )
            ),
            Err(AccessTokenValidatorError::InvalidToken(_)) =>
                Err(AccessGranterError::InvalidToken),
            Err(AccessTokenValidatorError::ExpiredToken(parsed_token)) => Ok(
                SessionInfo::Expired(
                    KnownSession {
                        raw_token: token.to_owned(),
                        session_id: parsed_token.session_id,
                        username: parsed_token.username,
                    }
                )
            )
        }
    }

    async fn login_user(
        &self,
        username: &UsernameStr,
        password: &str,
    ) -> Result<LoginResult, AccessGranterError> {
        debug!("logging user \"{username}\" in");
        let response: LoginResponse = self.caller
            .execute(
                Command(
                    bindings::command::Command::Login(
                        LoginRequest {
                            username: username.to_owned(),
                            password: password.to_owned(),
                            session_kind: SessionKind::Api,
                        }.into()
                    )
                )
            )
            .await?
            .try_into()?;
        match response.0 {
            Ok(SuccessfulLogin::Api { access_token, refresh_token })
            => Ok(LoginResult {
                refresh_token,
                access_token,
            }),

            Ok(_) => {
                error!("received invalid login response");
                Err(AccessGranterError::AuthDaemonInternalError)
            },

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
                Command(
                    bindings::command::Command::RefreshToken(
                        RefreshTokenRequest {
                            username: username.to_owned(),
                            refresh_token: refresh_token.to_owned(),
                        }.into()
                    )
                )
            )
            .await?
            .try_into()?;
        match response.0 {
            Ok(SuccessfulLogin::Api { access_token, refresh_token })
            => Ok(LoginResult {
                refresh_token,
                access_token,
            }),

            Ok(_) => {
                error!("received invalid login response");
                Err(AccessGranterError::AuthDaemonInternalError)
            },

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
        access_token: &str,
    ) -> Result<(), AccessGranterError> {
        trace!("deleting session for token \"{access_token}\"");
        let response: LogoutResponse = self.caller
            .execute(
                Command(
                    bindings::command::Command::Logout(
                        LogoutRequest {
                            access_token: access_token.to_owned(),
                            xsrf_token: None,
                        }.into()
                    )
                )
            )
            .await?
            .try_into()?;
        match response.0 {
            Some(error) => Err(
                match error {
                    LogoutError::LogoutInvalidCredentials => AccessGranterError::InvalidToken,
                    LogoutError::LogoutInternalError => AccessGranterError::AuthDaemonInternalError,
                }
            ),
            None => Ok(())
        }
    }
}
