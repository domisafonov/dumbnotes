use access_token::{AccessTokenData, AccessTokenValidator};
use dumbnotes::check_access_token;
use log::{error, trace};
use storage_ipc_data::bindings;
use bindings::StorageError;
use storage_ipc_data::model::get_note_details::{GetNoteDetailsRequest, GetNoteDetailsResponse};
use thiserror::Error;

use crate::StorageError as SE;
use crate::storage::NoteStorage;

pub async fn process_get_note_details(
    note_storage: &NoteStorage,
    access_token_validator: &AccessTokenValidator,
    request: GetNoteDetailsRequest,
) -> bindings::response::Response {
    process_get_note_details_impl(
        note_storage,
        access_token_validator,
        request,
    ).await
        .unwrap_or_else(|e| {
            error!("error processing get note details request: {e}");
            GetNoteDetailsResponse::Error(StorageError::InternalError)
        })
        .into()
}

async fn process_get_note_details_impl(
    note_storage: &NoteStorage,
    access_token_validator: &AccessTokenValidator,
    request: GetNoteDetailsRequest,
) -> Result<GetNoteDetailsResponse, GetNoteDetailsError> {
    let GetNoteDetailsRequest { access_token, notes_metadata } = request;

    let AccessTokenData { username, .. } = check_access_token!(
        "get note details",
        access_token_validator,
        access_token,
        GetNoteDetailsResponse::Error(StorageError::InvalidCredentials),
    );

    trace!("getting {} note details for user \"{username}\"", notes_metadata.len());
    Ok(
        GetNoteDetailsResponse::Notes(
            note_storage.get_note_details(&username, notes_metadata).await?
        )
    )
}

#[derive(Debug, Error)]
enum GetNoteDetailsError {
    #[error("note storage error: {0}")]
    NoteStorage(#[from] SE),
}
