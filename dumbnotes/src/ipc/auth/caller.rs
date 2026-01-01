use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use futures::{pin_mut, Stream};
use log::{error, trace};
use prost::Message;
use scc::HashMap;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::net::unix::OwnedWriteHalf;
use tokio::net::UnixStream;
use tokio::sync::{oneshot, Mutex};
use tokio_stream::StreamExt;
use crate::bin_constants::IPC_MESSAGE_MAX_SIZE;
use crate::error_exit;
use crate::ipc::auth::{message_stream, protobuf};
use crate::protobuf::{MappingError, OptionExt, ProtobufRequestError};

pub trait Caller: Send + Sync + 'static {
    fn execute(
        &self,
        command: protobuf::command::Command,
    ) -> impl Future<Output = Result<protobuf::response::Response, CallerError>> + Send;
}

pub type ProductionCaller = CallerImpl;

pub struct CallerImpl {
    write_socket: Mutex<OwnedWriteHalf>,
    next_request_id: AtomicU64,
    read_task: tokio::task::AbortHandle,
    active_requests: Arc<HashMap<u64, oneshot::Sender<protobuf::Response>>>,
}

impl Drop for CallerImpl {
    fn drop(&mut self) {
        self.read_task.abort();
    }
}

impl ProductionCaller {
    pub async fn new(socket: UnixStream) -> ProductionCaller {
        let (read_socket, write_socket) = socket.into_split();
        let stored_active_requests = Arc::new(HashMap::new());
        let active_requests = stored_active_requests.clone();
        let responses = message_stream::stream(read_socket);
        let read_task = tokio::task::spawn(
            Self::process_responses(
                active_requests,
                responses,
            )
        );
        CallerImpl {
            write_socket: Mutex::new(write_socket),
            next_request_id: AtomicU64::new(0),
            read_task: read_task.abort_handle(),
            active_requests: stored_active_requests,
        }
    }
}

impl CallerImpl {
    async fn process_responses(
        active_requests: Arc<HashMap<u64, oneshot::Sender<protobuf::Response>>>,
        responses: impl Stream<Item=protobuf::Response>,
    ) {
        pin_mut!(responses);
        while let Some(response) = responses.next().await {
            trace!("received response: {response:?}");
            let command_id = response.command_id;
            let (_, sender) = active_requests.remove_sync(&command_id)
                .unwrap_or_else(||
                    error_exit!(
                        "received response to unknown request id {}",
                        command_id,
                    )
                );
            if sender.send(response).is_err() {
                error!(
                    "receiver for request id {} already dropped",
                    command_id,
                );
            }
        }
    }

    fn get_next_request_id(&self) -> u64 {
        self.next_request_id.fetch_add(1, Ordering::Relaxed)
    }
}

impl Caller for CallerImpl {
    async fn execute(
        &self,
        command: protobuf::command::Command,
    ) -> Result<protobuf::response::Response, CallerError> {
        trace!("executing command {command:?}");
        let request_id = self.get_next_request_id();
        let command = protobuf::Command {
            command_id: request_id,
            command: Some(command),
        };
        let command = &command.encode_to_vec();
        if command.len() > IPC_MESSAGE_MAX_SIZE {
            return Err(CallerError::MessageTooBig);
        }
        let (sender, receiver) = oneshot::channel::<protobuf::Response>();
        self.active_requests.insert_sync(request_id, sender)
            .unwrap_or_else(|_|
                error_exit!("found previous instance of request with id {request_id}")
            );
        let mut socket = self.write_socket.lock().await;
        socket.write_u64(command.len() as u64).await
            .inspect_err(|_| {
                self.active_requests.remove_sync(&request_id);
            })?;
        socket.write_all(command).await
            .unwrap_or_else(|e|
                error_exit!("failed marshalling an auth command: {e}")
            );
        let response = receiver.await?;
        trace!("successfully awaited response: {response:?}");
        Ok(
            response.response
                .ok_or_mapping_error(
                    MappingError::missing("response")
                )?
        )
    }
}

#[derive(Debug, Error)]
pub enum CallerError {
    #[error("protobuf error: {0}")]
    Protobuf(#[from] ProtobufRequestError),

    #[error("socket io error: {0}")]
    Io(#[from] io::Error),

    #[error("can't receive response for request")]
    Receive(#[from] oneshot::error::RecvError),

    #[error("IPC message too big")]
    MessageTooBig,
}
