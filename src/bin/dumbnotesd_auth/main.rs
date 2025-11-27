mod cli;
mod protobuf;

use crate::cli::CliConfig;
use async_stream::stream;
use clap::{crate_name, Parser};
use dumbnotes::access_token::AccessTokenGenerator;
use dumbnotes::bin_constants::IPC_MESSAGE_MAX_SIZE;
use dumbnotes::config::hasher_config::ProductionHasherConfigData;
use dumbnotes::error_exit;
use dumbnotes::file_watcher::ProductionFileWatcher;
use dumbnotes::hasher::{Hasher, ProductionHasher, ProductionHasherConfig};
use dumbnotes::logging::init_logging;
use dumbnotes::session_storage::ProductionSessionStorage;
use futures::{pin_mut, Stream};
use josekit::jwk::Jwk;
use log::{error, info};
use prost::Message;
use scc::HashSet;
use socket2::Socket;
use std::error::Error;
use std::os::fd::{FromRawFd, IntoRawFd};
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::UnixStream;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() {
    init_logging();

    info!("{} starting up", crate_name!());

    let config = CliConfig::parse();

    let (read_socket, write_socket) = make_sockets(&config);

    let watcher = ProductionFileWatcher::new()
        .unwrap_or_else(|e| error_exit!("failed to create file watcher: {e}"));
    let session_storage = Box::new(
        ProductionSessionStorage
            ::new(
                &config.data_directory,
                watcher,
            )
            .await
            .unwrap_or_else(|e|
                error_exit!("could not initialize the session DB: {e}")
            )
    );

    let result = tokio::spawn(
        process_commands(
            make_hasher(&config),
            make_token_generator(&config),
            make_command_stream(read_socket),
            write_socket,
        )
    ).await;
    if let Err(e) = result {
        error_exit!("event loop finished with error: {e}")
    }

    info!("{} terminating normally", crate_name!());
}

async fn process_commands(
    hasher: impl Hasher,
    token_generator: AccessTokenGenerator,
    commands: impl Stream<Item=protobuf::Command>,
    write_socket: Arc<OwnedWriteHalf>,
) {
    info!("{} listening to commands", crate_name!());

    let mut active_request_ids = HashSet::<u64>::new();

    pin_mut!(commands);
    while let Some(command) = commands.next().await {
        if active_request_ids.insert_sync(command.command_id).is_err() {
            error!("duplicate command id: {}", command.command_id);
            continue
        }
        let command = match command.command {
            Some(command) => command,
            None => {
                error!("empty command with id {}", command.command_id);
                active_request_ids.remove_sync(&command.command_id);
                continue
            },
        };

        use protobuf::command::Command as CE;
        match command {
            CE::Login(request) => todo!(),
            CE::RefreshToken(request) => todo!(),
            CE::Logout(request) => todo!(),
        }
    }
}

fn make_command_stream(
    socket: OwnedReadHalf,
) -> impl Stream<Item=protobuf::Command> {
    let mut socket = BufReader::new(socket);
    let mut buffer = [0; IPC_MESSAGE_MAX_SIZE];
    stream! {
        let message_size = socket.read_u64().await.unwrap_or_else(|e| error_exit!("aaaaa"));
        let message_size = usize::try_from(message_size).unwrap_or_else(|e| error_exit!("aaaaaaa"));
        if message_size > IPC_MESSAGE_MAX_SIZE {
            error_exit!("message too big: {message_size}")
        }
        let buffer = &mut buffer[..message_size];
        socket.read_exact(buffer).await.unwrap_or_else(|e| error_exit!("aaaaa"));
        let command = protobuf::Command::decode(buffer.as_ref()).unwrap_or_else(|e| error_exit!("aaaaa"));
        // TODO
        yield command
    }
}

fn make_hasher(
    config: &CliConfig,
) -> impl Hasher + 'static {
    let hasher_config: ProductionHasherConfigData = serde_json
    ::from_str(&config.hasher_config)
        .unwrap_or_else(|e|
            error_exit!("hasher config is invalid: {e}")
        );
    let hasher_config: argon2::Params = hasher_config.clone().try_into().unwrap_or_else(|e| {
        error_exit!("hasher config read failed: {e}")
    });
    ProductionHasher::new(
        ProductionHasherConfig::new(hasher_config),
    )
}

fn make_sockets(
    config: &CliConfig,
) -> (OwnedReadHalf, Arc<OwnedWriteHalf>) {
    let command_socket = unsafe { Socket::from_raw_fd(config.socket_fd) };
    command_socket.set_cloexec(true)
        .unwrap_or_else(|e| error_exit!("failed command socket setup: {}", e));
    let command_socket = UnixStream
    ::from_std(
        unsafe { StdUnixStream::from_raw_fd(command_socket.into_raw_fd()) },
    )
        .unwrap_or_else(|e| error_exit!("failed command socket setup: {}", e));
    let (read_socket, write_socket) = command_socket.into_split();
    let write_socket = Arc::new(write_socket);
    (read_socket, write_socket)
}

fn make_token_generator(
    config: &CliConfig,
) -> AccessTokenGenerator {
    let jwt_private_key = read_jwt_key(&config.private_key_file)
        .unwrap_or_else(|e|
            error_exit!("failed reading the private jwt key: {e}")
        );
    AccessTokenGenerator::from_jwk(&jwt_private_key)
        .unwrap_or_else(|e|
            error_exit!("could not initialize access token generator: {e}")
        )
}

fn read_jwt_key(
    path: &Path,
) -> Result<Jwk, Box<dyn Error>> {
    test_permissions(
        path,
        |p| p == 0o600 || p == 0o400,
        &format!(
            "{} must be owned by root and have mode of 600 or 400",
            path.to_string_lossy(),
        )
    )?;
    test_permissions(
        path.parent().expect("path has no parent"),
        |p| p & 0o022 == 0,
        &format!(
            "{} must be owned by root and not be writeable by group or other",
            path.to_string_lossy(),
        ),
    )?;
    Ok(
        Jwk::from_bytes(
            std::fs::read(path)?
        )?
    )
}

#[cfg(not(debug_assertions))]
fn test_permissions(
    path: &Path,
    is_valid: impl FnOnce(u32) -> bool,
    message: &str,
) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let metadata = std::fs::metadata(path)?;
    let permissions = metadata.permissions().mode() & 0o777;
    if metadata.uid() != 0 || !is_valid(permissions) {
        error_exit!("{message}")
    }
    Ok(())
}

#[cfg(debug_assertions)]
fn test_permissions(
    _path: &Path,
    _is_valid: impl FnOnce(u32) -> bool,
    _message: &str,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}
