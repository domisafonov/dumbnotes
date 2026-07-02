use access_token::{AccessTokenData, AccessTokenValidator};
use dumbnotes::check_access_token;
use log::{error, trace};
use storage_ipc_data::model::read_note::{ReadNoteRequest, ReadNoteResponse};
use thiserror::Error;
use storage_ipc_data::bindings;
use bindings::StorageError;

use crate::StorageError as SE;
use crate::storage::NoteStorage;

pub async fn process_read_note(
    note_storage: &NoteStorage,
    access_token_validator: &AccessTokenValidator,
    request: ReadNoteRequest,
) -> bindings::response::Response {
    process_read_note_impl(
        note_storage,
        access_token_validator,
        request,
    ).await
        .unwrap_or_else(|e| {
            error!("error processing write note request: {e}");
            ReadNoteResponse(Err(StorageError::InternalError))
        })
        .into()
}

async fn process_read_note_impl(
    note_storage: &NoteStorage,
    access_token_validator: &AccessTokenValidator,
    request: ReadNoteRequest,
) -> Result<ReadNoteResponse, ReadNoteError> {
    let ReadNoteRequest { access_token, note_id } = request;

    let AccessTokenData { username, .. } = check_access_token!(
        "read note",
        access_token_validator,
        access_token,
        ReadNoteResponse(Err(StorageError::InvalidCredentials)),
    );

    trace!("reading note \"{note_id}\" for user \"{username}\"");
    match note_storage.read_note(&username, note_id).await {
        Ok(note) => Ok(ReadNoteResponse(Ok(note))),
        Err(SE::TooBig) => Ok(ReadNoteResponse(Err(StorageError::TooBig))),
        Err(SE::NoteNotFound) => Ok(ReadNoteResponse(Err(StorageError::NotFound))),
        Err(e) => Err(e.into()),
    }
}

#[derive(Debug, Error)]
enum ReadNoteError {
    #[error("note storage error: {0}")]
    NoteStorage(#[from] SE),
}
