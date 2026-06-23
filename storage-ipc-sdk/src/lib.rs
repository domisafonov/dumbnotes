pub mod errors;

use std::marker::PhantomData;

use ::data::{Note, NoteInfo};
use dumbnotes::{bin_constants::IPC_STORAGE_MESSAGE_MAX_SIZE, gen_proto_ipc_wrappers, ipc::{caller::{Caller, CallerImpl}, data::IpcOutput}};
use log::{error, warn};
use rocket::async_trait;
use storage_ipc_data::{bindings, model::{delete_note::{DeleteNoteRequest, DeleteNoteResponse}, get_note_details::{GetNoteDetailsRequest, GetNoteDetailsResponse}, list_notes::{ListNotesRequest, ListNotesResponse}, read_note::{ReadNoteRequest, ReadNoteResponse}, write_note::{WriteNoteRequest, WriteNoteResponse}}};
use tokio::{net::UnixStream, sync::oneshot};
use uuid::Uuid;

use crate::errors::StorageAccessorError;

#[async_trait]
pub trait StorageAccessor: Send + Sync + 'static {
    async fn get_users_notes(
        &self,
        access_token: String,
    ) -> Result<Vec<NoteInfo>, StorageAccessorError>;

    async fn get_note(
        &self,
        access_token: String,
        note_id: Uuid,
    ) -> Result<Note, StorageAccessorError>;

    async fn write_note(
        &self,
        access_token: String,
        note: Note,
    ) -> Result<(), StorageAccessorError>;

    async fn delete_note(
        &self,
        access_token: String,
        note_id: Uuid,
    ) -> Result<(), StorageAccessorError>;
}

pub struct StorageAccessorImpl<
    Command: Send + Sync + 'static,
    CommandContainer: prost::Message + 'static,
    CommandWrapper: IpcOutput<Command, CommandContainer>,
    Response: Send + Sync + 'static,
    C: Caller<Command, CommandContainer, CommandWrapper, Response>,
> {
    caller: C,
    _phantom: PhantomData<(Command, CommandContainer, CommandWrapper, Response)>,
}

type ProductionCaller = CallerImpl<
    bindings::response::Response,
    bindings::Response,
    Response,
>;

pub type ProductionStorageAccessor = StorageAccessorImpl<
    bindings::command::Command,
    bindings::Command,
    Command,
    bindings::response::Response,
    ProductionCaller,
>;

gen_proto_ipc_wrappers!(
    bindings::Response[response] | bindings::response::Response => pub Response,
    bindings::Command[command] | bindings::command::Command => pub Command,
);

impl ProductionStorageAccessor {
    pub async fn new(storage_socket: UnixStream) -> (Self, oneshot::Receiver<()>) {
        let (caller, shutdown_notice) = ProductionCaller
            ::new(storage_socket, IPC_STORAGE_MESSAGE_MAX_SIZE)
            .await;
        (
            StorageAccessorImpl {
                caller,
                _phantom: Default::default(),
            },
            shutdown_notice,
        )
    }
}

#[async_trait]
impl<
    C: Caller<
        bindings::command::Command,
        bindings::Command,
        Command,
        bindings::response::Response
    >,
> StorageAccessor for StorageAccessorImpl<bindings::command::Command, bindings::Command, Command, bindings::response::Response, C> {
    async fn get_users_notes(
        &self,
        access_token: String,
    ) -> Result<Vec<NoteInfo>, StorageAccessorError> {
        let response: ListNotesResponse = self.caller
            .execute(
                Command(
                    bindings::command::Command::ListNotes(
                        ListNotesRequest {
                            access_token: access_token.clone(),
                        }.into()
                    )
                )
            )
            .await?
            .try_into()?;
        let notes_metadata = match response {
            ListNotesResponse::Notes(notes_metadata) => notes_metadata,
            ListNotesResponse::Error(e) => return Err(e.into()),
        };
        let response: GetNoteDetailsResponse = self.caller
            .execute(
                Command(
                    bindings::command::Command::GetNoteDetails(
                        GetNoteDetailsRequest {
                            access_token,
                            notes_metadata,
                        }.into()
                    )
                )
            )
            .await?
            .try_into()?;
        match response {
            GetNoteDetailsResponse::Notes(notes_info) => Ok(
                notes_info
                    .into_iter()
                    .filter_map(|maybe_info| {
                        if maybe_info.is_none() {
                            warn!("no info could be read for a note");
                        }
                        maybe_info
                    })
                    .collect()
            ),
            GetNoteDetailsResponse::Error(e) => {
                error!("error fetching note info: {e:?}");
                Err(e.into())
            },
        }
    }

    async fn get_note(
        &self,
        access_token: String,
        note_id: Uuid,
    ) -> Result<Note, StorageAccessorError> {
        let response: ReadNoteResponse = self.caller
            .execute(
                Command(
                    bindings::command::Command::ReadNote(
                        ReadNoteRequest {
                            access_token,
                            note_id,
                        }.into()
                    )
                )
            )
            .await?
            .try_into()?;
        Ok(response.0?)
    }

    async fn write_note(
        &self,
        access_token: String,
        note: Note,
    ) -> Result<(), StorageAccessorError> {
        let response: WriteNoteResponse = self.caller
            .execute(
                Command(
                    bindings::command::Command::WriteNote(
                        WriteNoteRequest {
                            access_token,
                            note,
                        }.into()
                    )
                )
            )
            .await?
            .try_into()?;
        match response.0 {
            Some(e) => Err(e.into()),
            None => Ok(()),
        }
    }

    async fn delete_note(
        &self,
        access_token: String,
        note_id: Uuid,
    ) -> Result<(), StorageAccessorError> {
        let response: DeleteNoteResponse = self.caller
            .execute(
                Command(
                    bindings::command::Command::DeleteNote(
                        DeleteNoteRequest {
                            access_token,
                            note_id,
                        }.into()
                    )
                )
            )
            .await?
            .try_into()?;
        match response.0 {
            None => Ok(()),
            Some(e) => Err(e.into()),
        }
    }
}
