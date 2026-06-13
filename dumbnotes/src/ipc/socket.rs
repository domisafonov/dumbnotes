use std::{io, os::fd::{FromRawFd, OwnedFd, RawFd}};
use socket2::{Domain, Socket, Type};
use tokio::net::{UnixStream, unix::{OwnedReadHalf, OwnedWriteHalf}};
use util::error_exit;
use std::os::unix::net::UnixStream as StdUnixStream;

pub fn create_socket_pair() -> Result<(UnixStream, OwnedFd), std::io::Error> {
    Socket
        ::pair_raw(Domain::UNIX, Type::STREAM, None)
        .and_then(|(socket_to_child, childs_socket)| {
            socket_to_child.set_nonblocking(true)?;
            socket_to_child.set_cloexec(true)?;
            childs_socket.set_nonblocking(true)?;

            #[cfg(target_os = "macos")] {
                socket_to_child.set_nosigpipe(true)?;
                childs_socket.set_nosigpipe(true)?;
            }

            let socket_to_child = UnixStream::from_std(
                StdUnixStream::from(socket_to_child)
            )?;
            let childs_socket = OwnedFd::from(childs_socket);
            Ok((socket_to_child, childs_socket))
        })
}

pub fn create_socket_pairs(
    count: usize,
) -> Result<Vec<(UnixStream, OwnedFd)>, std::io::Error> {
    (0..count)
        .map(|_| create_socket_pair())
        .collect::<Result<_, _>>()
}

pub fn discover_socket_pair(
    socket_fd: RawFd,
) -> (OwnedReadHalf, OwnedWriteHalf) {
    fn make(socket_fd: RawFd) -> Result<(OwnedReadHalf, OwnedWriteHalf), io::Error> {
        let command_socket = unsafe { Socket::from_raw_fd(socket_fd) };
        command_socket.set_cloexec(true)?;
        let command_socket = UnixStream::from_std(
            StdUnixStream::from(command_socket),
        )?;
        Ok(command_socket.into_split())
    }
    make(socket_fd)
        .unwrap_or_else(|e|
            error_exit!("failed control socket setup: {}", e)
        )
}
