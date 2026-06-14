use dumbnotes::access_token::{AccessTokenData, AccessTokenValidator};
use dumbnotes::check_access_token;
use log::{error, trace};
use storage_ipc_data::model::list_notes::{ListNotesRequest, ListNotesResponse};
use thiserror::Error;
use storage_ipc_data::bindings;
use bindings::StorageError;

use crate::StorageError as SE;
use crate::storage::NoteStorage;

pub async fn process_list_notes(
    note_storage: &NoteStorage,
    access_token_validator: &AccessTokenValidator,
    request: ListNotesRequest,
) -> storage_ipc_data::bindings::response::Response {
    process_list_notes_impl(
        note_storage,
        access_token_validator,
        request,
    ).await
        .unwrap_or_else(|e| {
            error!("error processing list notes request: {e}");
            ListNotesResponse::Error(StorageError::InternalError)
        })
        .into()
}

async fn process_list_notes_impl(
    note_storage: &NoteStorage,
    access_token_validator: &AccessTokenValidator,
    request: ListNotesRequest,
) -> Result<ListNotesResponse, ListNotesError> {
    let ListNotesRequest { access_token } = request;

    let AccessTokenData { username, .. } = check_access_token!(
        "list notes",
        access_token_validator,
        access_token,
        ListNotesResponse::Error(StorageError::InvalidCredentials),
    );

    trace!("listing notes for user \"{username}\"");
    Ok(
        ListNotesResponse::Notes(note_storage.list_notes(&username).await?)
    )
}

#[derive(Debug, Error)]
enum ListNotesError {
    #[error("note storage error: {0}")]
    NoteStorage(#[from] SE),
}
