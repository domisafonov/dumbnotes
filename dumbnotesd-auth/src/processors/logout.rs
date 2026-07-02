use access_token::{AccessTokenData, AccessTokenValidator};
use data::SessionKind;
use dumbnotes::check_access_token;
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
    let LogoutRequest { access_token, xsrf_token } = request;

    let AccessTokenData { session_id, .. } = check_access_token!(
        "logout",
        access_token_validator,
        access_token,
        match xsrf_token {
            Some(_) => SessionKind::Web,
            None => SessionKind::Api,
        },
        LogoutResponse(Some(LogoutError::LogoutInvalidCredentials)),
    );

    debug!("deleting session {session_id}");
    let did_exist = session_storage
        .delete_session(session_id, xsrf_token)
        .await?;
    if did_exist {
        info!("session {session_id} deleted");
    } else {
        warn!("attempting to delete nonexistent session {session_id}");
    }
    Ok(LogoutResponse(None))
}

#[derive(Debug, Error)]
enum LogoutProcessorError {
    #[error("session storage error: {0}")]
    SessionStorage(#[from] SessionStorageError),
}
