use dumbnotes::access_token::{AccessTokenData, AccessTokenValidator, AccessTokenValidatorError};
use log::{debug, error, info, warn};
use thiserror::Error;
use crate::session_storage::{SessionStorage, SessionStorageError};
use auth_ipc_data::model::logout::{LogoutRequest, LogoutResponse};
use auth_ipc_data::bindings::LogoutError;

pub async fn process_logout(
    session_storage: &impl SessionStorage,
    access_token_validator: &AccessTokenValidator,
    request: LogoutRequest,
) -> auth_ipc_data::bindings::response::Response {
    process_logout_impl(session_storage, access_token_validator, request)
        .await
        .unwrap_or_else(|e| {
            error!("error processing logout request: {}", e);
            LogoutResponse(Some(LogoutError::LogoutInternalError))
        })
        .into()
}

async fn process_logout_impl(
    session_storage: &impl SessionStorage,
    access_token_validator: &AccessTokenValidator,
    request: LogoutRequest,
) -> Result<LogoutResponse, LogoutProcessorError> {
    let LogoutRequest { access_token } = request;

    match access_token_validator.check_access_token(&access_token) {
        Ok(AccessTokenData { session_id, .. }) => {
            debug!("deleting session {session_id}");
            let did_exist = session_storage
                .delete_session(session_id)
                .await?;
            if did_exist {
                info!("session {session_id} deleted");
            } else {
                warn!("attempting to delete nonexistent session {session_id}");
            }
            Ok(LogoutResponse(None))
        },
        Err(AccessTokenValidatorError::InvalidToken(_)) => {
            warn!("attempted logout using invalid access token: {access_token}");
            Ok(LogoutResponse(Some(LogoutError::LogoutInvalidCredentials)))
        },
        Err(AccessTokenValidatorError::ExpiredToken(_)) => {
            warn!("attempted logout using expired token");
            Ok(LogoutResponse(Some(LogoutError::LogoutInvalidCredentials)))
        },
    }
}

#[derive(Debug, Error)]
enum LogoutProcessorError {
    #[error("session storage error: {0}")]
    SessionStorage(#[from] SessionStorageError),
}
