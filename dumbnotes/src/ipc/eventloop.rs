use std::{io, sync::Arc};

use futures::{Stream, StreamExt, pin_mut};
use log::{debug, error, info, trace, warn};
use protobuf_common::ProtobufRequestError;
use scc::HashSet;
use thiserror::Error;
use tokio::{io::AsyncWriteExt, net::unix::OwnedWriteHalf, sync::Mutex};
use util::error_exit;
use crate::{ipc::data::{IpcInputContainerWrapper, IpcOutput, LoopInputMessage}, lib_constants::BIN_SHUTDOWN_TIMEOUT};

pub async fn process_commands<
    Command: Send + Sync + 'static,
    CommandContainer: prost::Message,
    LCommand,
    CommandStream: Stream<Item = LoopInputMessage<LCommand>>,
    State: Send + Sync + 'static,
    Dispatcher,
    DispatcherOutput,
    DispatcherError: std::error::Error + Send + Sync + 'static,
    Response: Send + Sync + 'static,
    LResponse: IpcOutput<Response, LResponseWrapper>,
    LResponseWrapper: prost::Message + 'static,
>(
    loop_name: impl AsRef<str>,
    commands: CommandStream,
    state: Arc<State>,
    write_socket: OwnedWriteHalf,
    dispatcher: Dispatcher,
    max_message_len: usize,
)
where
    LCommand: IpcInputContainerWrapper<Command, CommandContainer>
        + std::fmt::Debug,
    Dispatcher: Fn(
        Command,
        Arc<State>,
    ) -> DispatcherOutput
        + Send
        + Sync
        + 'static,
    DispatcherOutput: Future<
        Output = Result<LResponse, DispatcherError>
    > + Send + 'static,
{
    info!("{} listening to commands", loop_name.as_ref());

    struct InnerState {
        write_socket: Mutex<OwnedWriteHalf>,
        active_request_ids: HashSet<u64>,
    }

    let owned_inner_state = Arc::new(
        InnerState {
            write_socket: Mutex::new(write_socket),
            active_request_ids: HashSet::<u64>::new(),
        }
    );
    let owned_dispatcher = Arc::new(dispatcher);
    let owned_state = state;

    pin_mut!(commands);
    while let Some(l_command) = commands.next().await {
        let l_command = match l_command {
            LoopInputMessage::Shutdown => {
                debug!("shutting down by request");
                break
            },
            LoopInputMessage::UserMessage(message) => message,
        };

        trace!("received command: {l_command:?}");

        let (command_id, command) = l_command.into_id_and_input();

        if owned_inner_state.active_request_ids.insert_sync(command_id).is_err() {
            error!("duplicate command id: {}", command_id);
            continue
        }
        let command = match command {
            Ok(command) => command,
            Err(e) => {
                error!("failed to parse command with id {command_id}: {e}");
                owned_inner_state.active_request_ids.remove_sync(&command_id);
                continue
            },
        };

        let state = owned_state.clone();
        let owned_dispatcher = owned_dispatcher.clone();
        let inner_state = owned_inner_state.clone();
        tokio::spawn(async move {
            match owned_dispatcher(command, state).await {
                Ok(response) => {
                    debug!("command {command_id} executed successfully");
                    write_response(
                        &mut *inner_state.write_socket.lock().await,
                        response.into_container(command_id),
                        max_message_len,
                    ).await
                        .unwrap_or_else(|e|
                            error_exit!("error writing to the control socket: {e}")
                        );
                },
                Err(e) => {
                    error!("failed executing command {command_id}: {e}");
                }
            };

            inner_state.active_request_ids.remove_sync(&command_id);
        });
    }

    let active_count = owned_inner_state.active_request_ids.len();
    if active_count != 0 {
        debug!("command connection closed");
        return
    }

    warn!("waiting for {active_count} active requests to finish");
    tokio::time::sleep(BIN_SHUTDOWN_TIMEOUT).await;
    let active_count = owned_inner_state.active_request_ids.len();
    if active_count != 0 {
        error!("dropping {active_count} active requests after timeout");
    }
}

async fn write_response(
    write_socket: &mut OwnedWriteHalf,
    response: impl prost::Message,
    max_message_len: usize,
) -> Result<(), DispatchCommandError> {
    let response = response.encode_to_vec();
    if response.len() > max_message_len {
        return Err(
            DispatchCommandError::MessageTooBig {
                length: response.len(),
                max: max_message_len,
            }
        );
    }
    write_socket.write_u64(response.len() as u64).await?;
    write_socket.write_all(response.as_slice()).await?;
    Ok(())
}

#[derive(Debug, Error)]
enum DispatchCommandError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("IPC message too big: length is {length}, max is {max}")]
    MessageTooBig {
        length: usize,
        max: usize,
    },

    #[error(transparent)]
    Protobuf(#[from] ProtobufRequestError),
}
