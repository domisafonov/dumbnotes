use log::{debug, error, info, warn};
use thiserror::Error;
use time::OffsetDateTime;
use crate::session_storage::{SessionStorage, SessionStorageError};
use dumbnotes::ipc::auth::model::refresh_token::{RefreshTokenRequest, RefreshTokenResponse};
use dumbnotes::ipc::auth::model::successful_login::SuccessfulLogin;
use dumbnotes::ipc::auth::protobuf;
use dumbnotes::ipc::auth::protobuf::LoginError;
use crate::access_token_generator::AccessTokenGenerator;
use crate::access_token_generator::errors::AccessTokenGeneratorError;
use crate::app_constants::ACCESS_TOKEN_VALIDITY_TIME;

pub async fn process_refresh_token(
    session_storage: &impl SessionStorage,
    token_generator: &AccessTokenGenerator,
    request: RefreshTokenRequest,
) -> protobuf::response::Response {
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
    let RefreshTokenRequest { username, refresh_token} = request;
    debug!("refreshing access token for user \"{username}\"");
    let session = session_storage
        .get_session_by_token(&refresh_token)
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
            now + ACCESS_TOKEN_VALIDITY_TIME,
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
        )?;
    Ok(
        RefreshTokenResponse(
            Ok(
                SuccessfulLogin {
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
