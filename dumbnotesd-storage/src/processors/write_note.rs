use log::{error, trace};
use storage_ipc_data::bindings::StorageError;
use storage_ipc_data::model::write_note::{WriteNoteRequest, WriteNoteResponse};
use thiserror::Error;

use crate::storage::NoteStorage;
use crate::StorageError as SE;

pub async fn process_write_note(
    note_storage: &NoteStorage,
    request: WriteNoteRequest,
) -> storage_ipc_data::bindings::response::Response {
    process_write_note_impl(
        note_storage,
        request,
    ).await
        .unwrap_or_else(|e| {
            error!("error processing read note request: {e}");
            WriteNoteResponse(Some(StorageError::InternalError))
        })
        .into()
}

async fn process_write_note_impl(
    note_storage: &NoteStorage,
    request: WriteNoteRequest,
) -> Result<WriteNoteResponse, WriteNoteError> {
    let WriteNoteRequest { username, note } = request;
    trace!("writing note \"{note:?}\" for user \"{username}\"");
    match note_storage.write_note(&username, &note).await {
        Ok(()) => Ok(WriteNoteResponse(None)),
        Err(SE::TooBig) => Ok(WriteNoteResponse(Some(StorageError::TooBig))),
        Err(SE::NoteNotFound) => Ok(WriteNoteResponse(Some(StorageError::NotFound))),
        Err(e) => Err(e.into()),
    }
}

#[derive(Debug, Error)]
enum WriteNoteError {
    #[error("note storage error: {0}")]
    NoteStorage(#[from] SE),
}
