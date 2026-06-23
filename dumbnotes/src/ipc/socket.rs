use std::{io, os::fd::{FromRawFd, OwnedFd, RawFd}};
use socket2::{Domain, Socket, Type};
use tokio::net::UnixStream;
use util::error_exit;
use std::os::unix::net::UnixStream as StdUnixStream;

pub fn create_socket_pair() -> Result<(Socket, Socket), std::io::Error> {
    Socket
        ::pair_raw(Domain::UNIX, Type::STREAM, None)
        .and_then(|(cloexec_socket, immediate_use_socket)| {
            cloexec_socket.set_nonblocking(true)?;
            cloexec_socket.set_cloexec(true)?;
            immediate_use_socket.set_nonblocking(true)?;

            #[cfg(target_os = "macos")] {
                cloexec_socket.set_nosigpipe(true)?;
                immediate_use_socket.set_nosigpipe(true)?;
            }

            Ok((
                cloexec_socket,
                immediate_use_socket,
            ))
        })
}

pub fn discover_socket(
    socket_fd: RawFd,
) -> UnixStream {
    fn make(socket_fd: RawFd) -> Result<UnixStream, io::Error> {
        let command_socket = unsafe { Socket::from_raw_fd(socket_fd) };
        command_socket.set_cloexec(true)?;
        let command_socket = UnixStream::from_std(
            StdUnixStream::from(command_socket),
        )?;
        Ok(command_socket)
    }
    make(socket_fd)
        .unwrap_or_else(|e|
            error_exit!("failed control socket setup: {}", e)
        )
}
