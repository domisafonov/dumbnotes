use std::io;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use futures::{pin_mut, Stream};
use log::{error, trace, warn};
use scc::HashMap;
use scopeguard::guard;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::net::unix::OwnedWriteHalf;
use tokio::net::UnixStream;
use tokio::sync::{oneshot, Mutex};
use tokio_stream::StreamExt;
use util::error_exit;
use crate::ipc::message_stream;
use crate::ipc::data::{IpcInputContainerWrapper, IpcOutput};
use protobuf_common::ProtobufRequestError;

// TODO: decouple from auth specifically
pub trait Caller<
    Command: Send + Sync + 'static,
    CommandContainer: prost::Message + 'static,
    CommandWrapper: IpcOutput<Command, CommandContainer>,
    Response: Send + Sync + 'static,
>: Send + Sync + 'static {
    fn execute(
        &self,
        command: CommandWrapper,
    ) -> impl Future<Output = Result<Response, CallerError>> + Send;
}

pub struct CallerImpl<
    Response: Send + Sync,
    ResponseContainer: prost::Message,
    ResponseContainerWrapper: IpcInputContainerWrapper<Response, ResponseContainer>,
> where
{
    write_socket: Mutex<OwnedWriteHalf>,
    next_request_id: AtomicU64,
    read_task: tokio::task::AbortHandle,
    active_requests: Arc<
        HashMap<u64, oneshot::Sender<ResponseContainerWrapper>>>,
    max_message_size: usize,
    _phantom: PhantomData<(Response, ResponseContainer)>,
}

impl<
    Response: Send + Sync,
    ResponseContainer: prost::Message,
    ResponseContainerWrapper: IpcInputContainerWrapper<Response, ResponseContainer>,
> Drop for CallerImpl<Response, ResponseContainer, ResponseContainerWrapper> {
    fn drop(&mut self) {
        self.read_task.abort();
    }
}

impl<
    Response: Send + Sync + 'static,
    ResponseContainer,
    ResponseContainerWrapper,
> CallerImpl<Response, ResponseContainer, ResponseContainerWrapper>
where
    ResponseContainer: prost::Message + Default + std::fmt::Debug + 'static,
    ResponseContainerWrapper: IpcInputContainerWrapper<Response, ResponseContainer>
        + std::fmt::Debug,
{
    pub async fn new(
        socket: UnixStream,
        max_message_size: usize,
    ) -> Self {
        let (read_socket, write_socket) = socket.into_split();
        let stored_active_requests = Arc::new(HashMap::new());
        let active_requests = stored_active_requests.clone();
        let responses = message_stream::stream(read_socket, max_message_size)
            .map(IpcInputContainerWrapper::wrap);
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
            max_message_size,
            _phantom: Default::default(),
        }
    }
}

impl<
    Response: Send + Sync,
    ResponseContainer,
    ResponseContainerWrapper,
> CallerImpl<Response, ResponseContainer, ResponseContainerWrapper>
where
    ResponseContainer: prost::Message + Default + std::fmt::Debug,
    ResponseContainerWrapper: IpcInputContainerWrapper<Response, ResponseContainer> + std::fmt::Debug,
{
    async fn process_responses(
        active_requests: Arc<HashMap<u64, oneshot::Sender<ResponseContainerWrapper>>>,
        responses: impl Stream<Item=ResponseContainerWrapper>,
    ) {
        pin_mut!(responses);
        while let Some(response) = responses.next().await {
            trace!("received response: {response:?}");
            let command_id = response.get_id();
            let Some((_, sender)) = active_requests.remove_sync(&command_id) else {
                warn!(
                    "received response to unknown or dropped request, id {}",
                    command_id,
                );
                continue;
            };
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

impl<
    Command: Send + Sync + 'static,
    CommandContainer: prost::Message + Default + 'static,
    CommandWrapper: IpcOutput<Command, CommandContainer> + std::fmt::Debug,
    Response: Send + Sync + 'static + std::fmt::Debug,
    ResponseContainer: prost::Message + Default + std::fmt::Debug + 'static,
    ResponseContainerWrapper,
> Caller<Command, CommandContainer, CommandWrapper, Response>
    for CallerImpl<Response, ResponseContainer, ResponseContainerWrapper>
where
    ResponseContainerWrapper: IpcInputContainerWrapper<Response, ResponseContainer>
        + std::fmt::Debug,
{
    async fn execute(
        &self,
        command: CommandWrapper,
    ) -> Result<Response, CallerError> {
        trace!("executing command {command:?}");
        let request_id = self.get_next_request_id();
        let command = command.into_container(request_id);
        let command = &command.encode_to_vec();
        if command.len() > self.max_message_size {
            return Err(
                CallerError::MessageTooBig {
                    length: command.len(),
                    max: self.max_message_size,
                }
            );
        }
        let (sender, receiver) = oneshot::channel::<ResponseContainerWrapper>();
        self.active_requests.insert_sync(request_id, sender)
            .unwrap_or_else(|_|
                error_exit!("found previous instance of request with id {request_id}")
            );
        let request_guard = guard(self.active_requests.clone(), |ar| {
            ar.remove_sync(&request_id);
        });
        let mut socket = self.write_socket.lock().await;
        socket.write_u64(command.len() as u64).await?;
        socket.write_all(command).await
            .unwrap_or_else(|e|
                error_exit!("failed marshalling an auth command: {e}")
            );
        let (_, response) = receiver.await?.into_id_and_input();
        drop(request_guard);
        let response = response?;
        trace!("successfully awaited response: {response:?}");
        Ok(response)
    }
}

#[derive(Debug, Error)]
pub enum CallerError {
    #[error("bindings error: {0}")]
    Protobuf(#[from] ProtobufRequestError),

    #[error("socket io error: {0}")]
    Io(#[from] io::Error),

    #[error("can't receive response for request")]
    Receive(#[from] oneshot::error::RecvError),

    #[error("IPC message too big: length is {length}, max is {max}")]
    MessageTooBig {
        length: usize,
        max: usize,
    },
}
