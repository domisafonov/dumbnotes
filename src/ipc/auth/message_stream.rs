use std::io;
use async_stream::stream;
use futures::Stream;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::net::unix::OwnedReadHalf;
use crate::bin_constants::IPC_MESSAGE_MAX_SIZE;
use crate::error_exit;

pub fn stream<I: prost::Message + Default>(
    socket: OwnedReadHalf,
) -> impl Stream<Item=I> {
    let mut socket = BufReader::new(socket);
    let mut buffer = [0; IPC_MESSAGE_MAX_SIZE];
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
        if message_size > IPC_MESSAGE_MAX_SIZE {
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
