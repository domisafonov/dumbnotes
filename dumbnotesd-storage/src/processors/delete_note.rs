use access_token::{AccessTokenData, AccessTokenValidator};
use dumbnotes::check_access_token;
use log::{error, trace};
use storage_ipc_data::model::delete_note::{DeleteNoteRequest, DeleteNoteResponse};
use storage_ipc_data::bindings;
use bindings::StorageError;
use thiserror::Error;

use crate::StorageError as SE;
use crate::storage::NoteStorage;

pub async fn process_delete_note(
    note_storage: &NoteStorage,
    access_token_validator: &AccessTokenValidator,
    request: DeleteNoteRequest,
) -> bindings::response::Response {
    process_delete_note_impl(
        note_storage,
        access_token_validator,
        request,
    ).await
        .unwrap_or_else(|e| {
            error!("error processing note delete request: {e}");
            DeleteNoteResponse(Some(StorageError::InternalError))
        })
        .into()
}

async fn process_delete_note_impl(
    note_storage: &NoteStorage,
    access_token_validator: &AccessTokenValidator,
    request: DeleteNoteRequest,
) -> Result<DeleteNoteResponse, DeleteNoteError> {
    let DeleteNoteRequest { access_token, note_id } = request;

    let AccessTokenData { username, .. } = check_access_token!(
        "delete note",
        access_token_validator,
        access_token,
        DeleteNoteResponse(Some(StorageError::InvalidCredentials)),
    );

    trace!("deleting note \"{note_id}\" for user \"{username}\"");
    match note_storage.delete_note(&username, note_id).await {
        Ok(()) => Ok(DeleteNoteResponse(None)),
        Err(SE::NoteNotFound) => Ok(DeleteNoteResponse(Some(StorageError::NotFound))),
        Err(e) => Err(e.into()),
    }
}

#[derive(Debug, Error)]
enum DeleteNoteError {
    #[error("note storage error: {0}")]
    NoteStorage(#[from] SE),
}
