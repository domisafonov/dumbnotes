use std::sync::Arc;

use clap::crate_name;
use dumbnotes::{bin_constants::IPC_STORAGE_MESSAGE_MAX_SIZE, gen_proto_ipc_wrappers, ipc::data::{LoopInputMessage, LoopStreamExt}};
use futures::stream::BoxStream;
use protobuf_common::ProtobufRequestError;
use storage_ipc_data::bindings;
use tokio::net::unix::OwnedWriteHalf;

use crate::{processors::{process_delete_note, process_get_note_details, process_list_notes, process_read_note, process_write_note}, storage::NoteStorage};

pub struct State {
    pub note_storage: NoteStorage,
}

pub async fn process_commands(
    state: Arc<State>,
    commands: BoxStream<'static, LoopInputMessage<bindings::Command>>,
    write_socket: OwnedWriteHalf,
) {
    use dumbnotes::ipc::eventloop::process_commands;

    process_commands(
        crate_name!(),
        commands.map_loop_message(Command),
        state,
        write_socket,
        dispatch_command,
        IPC_STORAGE_MESSAGE_MAX_SIZE,
    ).await;
}

async fn dispatch_command(
    command: bindings::command::Command,
    state: Arc<State>,
) -> Result<Response, ProtobufRequestError> {
    use bindings::command::Command as CE;
    let response = match command {
        CE::ReadNote(request) => process_read_note(
            &state.note_storage,
            request.try_into()?,
        ).await,
        CE::WriteNote(request) => process_write_note(
            &state.note_storage,
            request.try_into()?,
        ).await,
        CE::ListNotes(request) => process_list_notes(
            &state.note_storage,
            request.try_into()?
        ).await,
        CE::GetNoteDetails(request) => process_get_note_details(
            &state.note_storage,
            request.try_into()?,
        ).await,
        CE::DeleteNote(request) => process_delete_note(
            &state.note_storage,
            request.try_into()?,
        ).await,
    };
    Ok(Response(response))
}

gen_proto_ipc_wrappers!(
    bindings::Command[command] | bindings::command::Command => Command,
    bindings::Response[response] | bindings::response::Response => Response,
);
