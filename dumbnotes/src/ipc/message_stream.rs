use std::io;
use async_stream::stream;
use futures::Stream;
use tokio::io::AsyncReadExt;
use tokio::net::unix::OwnedReadHalf;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use tokio_stream::StreamExt;
use util::error_exit;

use crate::ipc::data::LoopInputMessage;

pub fn stream<I: prost::Message + Default>(
    mut socket: OwnedReadHalf,
    max_message_size: usize,
) -> impl Stream<Item=I> {
    let mut buffer = Vec::<u8>::new();
    buffer.resize(max_message_size, 0);
    stream! { loop {
        let message_size = match socket.read_u64().await {
            Ok(size) => size,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => error_exit!("failed to read message size: {e}"),
        };
        let message_size = usize::try_from(message_size)
            .unwrap_or_else(|e|
                error_exit!("read incorrect message size: {e}")
            );
        if message_size > max_message_size {
            error_exit!("message too big: {message_size}")
        }
        let buffer = &mut buffer[..message_size];
        socket.read_exact(buffer).await
            .unwrap_or_else(|e|
                error_exit!("error reading message: {e}")
            );
        let command = I::decode(buffer.as_ref())
            .unwrap_or_else(|e|
                error_exit!("error decoding message: {e}")
            );
        yield command
    } }
}

pub fn loop_input_stream<I: prost::Message + Default>(
    socket: OwnedReadHalf,
    max_message_size: usize,
) -> (
    UnboundedSender<LoopInputMessage<I>>,
    impl Stream<Item = LoopInputMessage<I>> + Sized,
) {
    let mut main_source = Box::pin(stream(socket, max_message_size));
    let (sender, mut side_source) = unbounded_channel::<LoopInputMessage<I>>();

    let stream = stream! { loop {
        tokio::select!(
            biased;
            injected = side_source.recv() => if let Some(injected) = injected {
                yield injected
            },
            main_message = main_source.next() => if let Some(main_message) = main_message {
                yield LoopInputMessage::UserMessage(main_message)
            } else {
                break
            },
            else => break,
        )
    } };
    (sender, stream)
}
