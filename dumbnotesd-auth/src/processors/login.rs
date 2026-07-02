use access_token::{AccessTokenGenerator, AccessTokenGeneratorError};
use data::{ApiSession, Session, SessionKind, WebSession};
use thiserror::Error;
use crate::app_constants::{API_ACCESS_TOKEN_VALIDITY_TIME, WEB_ACCESS_TOKEN_VALIDITY_TIME};
use crate::session_storage::{SessionStorage, SessionStorageError};
use crate::user_db::{UserDb, UserDbError};
use log::{debug, error, info, warn};
use time::OffsetDateTime;
use auth_ipc_data::model::login::{LoginRequest, LoginResponse};
use auth_ipc_data::model::successful_login::SuccessfulLogin;
use auth_ipc_data::bindings::LoginError;

pub async fn process_login(
    user_db: &impl UserDb,
    session_storage: &impl SessionStorage,
    token_generator: &AccessTokenGenerator,
    request: LoginRequest,
) -> auth_ipc_data::bindings::response::Response {
    process_login_impl(
        user_db,
        session_storage,
        token_generator,
        request,
    ).await
        .unwrap_or_else(|e| {
            error!("error processing login request: {e}");
            LoginResponse(Err(LoginError::LoginInternalError))
        })
        .into()
}

async fn process_login_impl(
    user_db: &impl UserDb,
    session_storage: &impl SessionStorage,
    token_generator: &AccessTokenGenerator,
    request: LoginRequest,
) -> Result<LoginResponse, LoginProcessorError> {
    let LoginRequest { username, password, session_kind } = request;
    let session_kind: SessionKind = session_kind.into();
    debug!("logging user \"{username}\" in");
    if user_db.check_user_credentials(&username, &password).await? {
        let now = OffsetDateTime::now_utc();
        let expires_at = match session_kind {
            SessionKind::Api => now + API_ACCESS_TOKEN_VALIDITY_TIME,
            SessionKind::Web => now + WEB_ACCESS_TOKEN_VALIDITY_TIME,
        };
        let session = session_storage
            .create_session(
                &username,
                now,
                expires_at,
                session_kind,
            )
            .await?;
        let access_token = token_generator
            .generate_token(
                session.get_session_id(),
                &session.get_username(),
                &now.into(),
                &expires_at.into(),
                session_kind,
            )?;
        info!(
            "logged user \"{username}\" in with session \"{}\"",
            session.get_session_id(),
        );
        Ok(
            LoginResponse(
                Ok(
                    match session {
                        Session::Api(ApiSession { refresh_token, .. })
                        => SuccessfulLogin::Api {
                            access_token,
                            refresh_token,
                        },

                        Session::Web(WebSession { xsrf_token, .. })
                        => SuccessfulLogin::Web {
                            access_token,
                            xsrf_token,
                        },
                    }
                )
            )
        )
    } else {
        warn!("invalid credentials for user \"{}\"", username);
        Ok(
            LoginResponse(
                Err(LoginError::LoginInvalidCredentials)
            )
        )
    }
}

#[derive(Debug, Error)]
enum LoginProcessorError {
    #[error("user database error: {0}")]
    UserDb(#[from] UserDbError),

    #[error("session storage error: {0}")]
    SessionStorage(#[from] SessionStorageError),

    #[error("error generating access token: {0}")]
    AccessTokenGenerator(#[from] AccessTokenGeneratorError),
}
