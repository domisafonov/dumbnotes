use std::{borrow::Borrow, os::fd::RawFd, sync::Arc};

use futures::{FutureExt, StreamExt, future::{join_all, select_all}, stream::BoxStream};
use log::{error, info};
use tokio::{net::unix::OwnedWriteHalf, time::Instant};

use crate::ipc::{data::LoopInputMessage, message_stream, socket::discover_socket};

pub async fn launch_event_loops<
    SocketContainer,
    Deps,
    MakeLoop,
    Loop,
    LoopInput,
>(
    crate_name: impl AsRef<str>,
    socket_fds: SocketContainer,
    create_deps: impl AsyncFn() -> Deps,
    mut make_loop: MakeLoop,
    max_message_size: usize,
    pre_spawn: impl FnOnce(),
    shutdown_timeout: std::time::Duration,
) -> i32
where
    SocketContainer: IntoIterator,
    SocketContainer::Item: Borrow<RawFd>,
    SocketContainer::IntoIter: ExactSizeIterator,
    MakeLoop: FnMut(
        Arc<Deps>,
        BoxStream<'static, LoopInputMessage<LoopInput>>,
        OwnedWriteHalf,
    ) -> Loop,
    Loop: Future<Output=()> + Send + 'static,
    LoopInput: prost::Message + Default + 'static,
{
    let socket_fds = socket_fds.into_iter();
    let n_sockets = socket_fds.len();
    let sockets = socket_fds
        .into_iter()
        .map(|fd| discover_socket(*fd.borrow()).into_split())
        .collect::<Vec<_>>();

    let deps = Arc::new(create_deps().await);

    let loops_builder = sockets
        .into_iter()
        .map(|(read_socket, write_socket)| {
            let (sender, stream) = message_stream::loop_input_stream::<LoopInput>(
                read_socket,
                max_message_size,
            );
            (
                tokio::spawn(
                    make_loop(deps.clone(), stream.boxed(), write_socket)
                        .boxed()
                ),
                sender,
            )
        });

    pre_spawn();

    let mut actual_loops = Vec::with_capacity(n_sockets);
    let mut senders = Vec::with_capacity(n_sockets);
    for (join_result, sender) in loops_builder {
        actual_loops.push(join_result);
        senders.push(sender);
    }
    drop(deps);

    let (res, _, actual_loops) = select_all(actual_loops).await;
    let mut is_ok = if let Err(e) = res {
        error!("an event loop exitted with error: \"{e}\", shutting down");
        false
    } else {
        info!("an event loop exitted");
        true
    };

    for s in senders.iter() {
        // ignoring already dropped receivers
        let _ = s.send(crate::ipc::data::LoopInputMessage::Shutdown);
    }

    let timeout = Instant::now() + shutdown_timeout;
    let timed_out_loops = actual_loops.into_iter()
        .map(|l| tokio::time::timeout_at(timeout, l));
    for timeout_result in join_all(timed_out_loops).await {
        match timeout_result {
            Ok(join_result) => if let Err(e) = join_result {
                error!("an event loop exitted with error {e}");
                is_ok = false
            },
            Err(_) => {
                error!("timeout reached waiting for an event loop to finish");
                is_ok = false
            },
        }
    }

    if is_ok {
        info!("{} terminating normally", crate_name.as_ref());
        0
    } else {
        1
    }
}
