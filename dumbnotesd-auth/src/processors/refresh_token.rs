use access_token::{AccessTokenGenerator, AccessTokenGeneratorError};
use data::SessionKind;
use log::{debug, error, info, warn};
use thiserror::Error;
use time::OffsetDateTime;
use crate::app_constants::API_ACCESS_TOKEN_VALIDITY_TIME;
use crate::session_storage::{SessionStorage, SessionStorageError};
use auth_ipc_data::model::refresh_token::{RefreshTokenRequest, RefreshTokenResponse};
use auth_ipc_data::model::successful_login::SuccessfulLogin;
use auth_ipc_data::bindings::LoginError;

pub async fn process_refresh_token(
    session_storage: &impl SessionStorage,
    token_generator: &AccessTokenGenerator,
    request: RefreshTokenRequest,
) -> auth_ipc_data::bindings::response::Response {
    process_refresh_token_impl(
        session_storage,
        token_generator,
        request,
    ).await
        .unwrap_or_else(|e| {
            error!("error processing refresh token request: {}", e);
            RefreshTokenResponse(Err(LoginError::LoginInternalError))
        })
        .into()
}

async fn process_refresh_token_impl(
    session_storage: &impl SessionStorage,
    token_generator: &AccessTokenGenerator,
    request: RefreshTokenRequest,
) -> Result<RefreshTokenResponse, RefreshTokenProcessorError> {
    let RefreshTokenRequest { username, refresh_token } = request;
    debug!("refreshing access token for user \"{username}\"");
    let session = session_storage
        .get_api_session_by_token(&refresh_token)
        .await?;
    if let Some(session) = session
        && session.username.as_username_str() != username.as_username_str()
    {
        warn!(
            "attempt to refresh access token for nonexisting \
                or mismatched user \"{username}\""
        );
        return Ok(
            RefreshTokenResponse(
                Err(LoginError::LoginInvalidCredentials)
            )
        )
    }
    let now = OffsetDateTime::now_utc();
    let session = session_storage
        .refresh_session(
            &refresh_token,
            now + API_ACCESS_TOKEN_VALIDITY_TIME,
        )
        .await;
    let session = match session {
        Ok(session) => session,
        Err(SessionStorageError::SessionNotFound) => return Ok(
            RefreshTokenResponse(
                Err(LoginError::LoginInvalidCredentials)
            )
        ),
        Err(e) => return Err(e.into()),
    };
    info!(
        "refreshed session {} for user \"{username}\"",
        session.session_id,
    );
    let access_token = token_generator
        .generate_token(
            session.session_id,
            &session.username,
            &now.into(),
            &session.expires_at.into(),
            SessionKind::Api,
        )?;
    Ok(
        RefreshTokenResponse(
            Ok(
                SuccessfulLogin::Api {
                    access_token,
                    refresh_token: session.refresh_token,
                }
            )
        )
    )
}

#[derive(Debug, Error)]
enum RefreshTokenProcessorError {
    #[error("session storage error: {0}")]
    SessionStorage(#[from] SessionStorageError),

    #[error("error generating access token: {0}")]
    AccessTokenGenerator(#[from] AccessTokenGeneratorError),
}
