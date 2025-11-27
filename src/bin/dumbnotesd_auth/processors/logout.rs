use log::{debug, error, info, warn};
use thiserror::Error;
use dumbnotes::session_storage::{SessionStorage, SessionStorageError};
use crate::model::logout::{LogoutRequest, LogoutResponse};
use crate::protobuf;
use crate::protobuf::LogoutError;

pub async fn process_logout(
    session_storage: &impl SessionStorage,
    request: LogoutRequest,
) -> protobuf::response::Response {
    process_logout_impl(session_storage, request)
        .await
        .unwrap_or_else(|e| {
            error!("error processing logout request: {}", e);
            LogoutResponse(Some(LogoutError::LogoutInternalError))
        })
        .into()
}

async fn process_logout_impl(
    session_storage: &impl SessionStorage,
    request: LogoutRequest,
) -> Result<LogoutResponse, LogoutProcessorError> {
    let LogoutRequest { session_id } = request;
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
}

#[derive(Debug, Error)]
enum LogoutProcessorError {
    #[error("session storage error: {0}")]
    SessionStorage(#[from] SessionStorageError),
}
