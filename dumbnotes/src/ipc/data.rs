use futures::Stream;
use protobuf_common::ProtobufRequestError;
use tokio_stream::StreamExt;

pub trait IpcInputContainerWrapper<
    T: Send + Sync,
    W: prost::Message + Sized,
>: Send + Sync + Sized + 'static {
    fn get_id(&self) -> u64;

    fn get_input(&self) -> Result<T, ProtobufRequestError>;

    fn into_id_and_input(self) -> (u64, Result<T, ProtobufRequestError>) {
        (self.get_id(), self.get_input())
    }

    fn wrap(wrapped: W) -> Self;
}

pub trait IpcOutput<
    T,
    W: prost::Message + Sized,
>: Send + Sync + 'static {
    fn into_container(self, command_id: u64) -> W;
}

pub enum LoopInputMessage<T> {
    UserMessage(T),
    Shutdown,
}

impl<T> LoopInputMessage<T> {
    pub fn map<U, F: FnMut(T) -> U>(self, mut f: F) -> LoopInputMessage<U> {
        match self {
            LoopInputMessage::UserMessage(m) => LoopInputMessage::UserMessage(f(m)),
            LoopInputMessage::Shutdown => LoopInputMessage::Shutdown,
        }
    }
}

pub trait LoopStreamExt<T, U>: Stream<Item = LoopInputMessage<T>> + Sized {
    fn map_loop_message<F: FnMut(T) -> U>(
        self,
        f: F,
    ) -> impl Stream<Item = LoopInputMessage<U>>;
}
impl<T, U, S> LoopStreamExt<T, U> for S
where
    S: Stream<Item = LoopInputMessage<T>>,
{
    fn map_loop_message<F: FnMut(T) -> U>(
        self,
        mut f: F,
    ) -> impl Stream<Item = LoopInputMessage<U>> {
        self.map(move |m| m.map(|inner| f(inner)))
    }
}
