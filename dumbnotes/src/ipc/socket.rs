use std::os::fd::OwnedFd;
use socket2::{Domain, Socket, Type};
use tokio::net::UnixStream;
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
