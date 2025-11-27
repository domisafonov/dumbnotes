mod cli;
mod protobuf;
mod model;
mod eventloop;
mod processors;

use crate::cli::CliConfig;
use async_stream::stream;
use clap::{crate_name, Parser};
use dumbnotes::access_token::AccessTokenGenerator;
use dumbnotes::bin_constants::IPC_MESSAGE_MAX_SIZE;
use dumbnotes::config::hasher_config::ProductionHasherConfigData;
use dumbnotes::error_exit;
use dumbnotes::file_watcher::ProductionFileWatcher;
use dumbnotes::hasher::{ProductionHasher, ProductionHasherConfig};
use dumbnotes::logging::init_logging;
use dumbnotes::session_storage::ProductionSessionStorage;
use dumbnotes::user_db::ProductionUserDb;
use futures::Stream;
use josekit::jwk::Jwk;
use log::info;
use prost::Message;
use socket2::Socket;
use std::error::Error;
use std::io;
use std::os::fd::{FromRawFd, IntoRawFd};
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::UnixStream;

#[tokio::main]
async fn main() {
    init_logging();

    info!("{} starting up", crate_name!());

    let config = CliConfig::parse();

    let (read_socket, write_socket) = make_sockets(&config);
    let watcher = ProductionFileWatcher::new()
        .unwrap_or_else(|e| error_exit!("failed to create file watcher: {e}"));
    let hasher = make_hasher(&config);

    let result = tokio::spawn(
        eventloop::process_commands(
            make_token_generator(&config),
            make_user_db(
                &config,
                hasher,
                watcher.clone(),
            ).await,
            make_session_storage(
                &config,
                watcher,
            ).await,
            make_command_stream(read_socket),
            write_socket,
        )
    ).await;

    if let Err(e) = result {
        error_exit!("event loop finished with error: {e}")
    }

    info!("{} terminating normally", crate_name!());
}

fn make_command_stream(
    socket: OwnedReadHalf,
) -> impl Stream<Item=protobuf::Command> {
    let mut socket = BufReader::new(socket);
    let mut buffer = [0; IPC_MESSAGE_MAX_SIZE];
    stream! {
        let message_size = socket.read_u64().await
            .unwrap_or_else(|e|
                error_exit!("failed to read message size: {e}")
            );
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
                error_exit!("error reading command: {e}")
            );
        let command = protobuf::Command::decode(buffer.as_ref())
            .unwrap_or_else(|e|
                error_exit!("error decoding command: {e}")
            );
        yield command
    }
}

fn make_hasher(
    config: &CliConfig,
) -> ProductionHasher {
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
) -> (OwnedReadHalf, OwnedWriteHalf) {
    fn make(config: &CliConfig) -> Result<(OwnedReadHalf, OwnedWriteHalf), io::Error> {
        let command_socket = unsafe { Socket::from_raw_fd(config.socket_fd) };
        command_socket.set_cloexec(true)?;
        let command_socket = UnixStream::from_std(
            unsafe { StdUnixStream::from_raw_fd(command_socket.into_raw_fd()) },
        )?;
        Ok(command_socket.into_split())
    }
    make(config)
        .unwrap_or_else(|e|
            error_exit!("failed control socket setup: {}", e)
        )
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

async fn make_user_db(
    config: &CliConfig,
    hasher: ProductionHasher,
    watcher: ProductionFileWatcher,
) -> ProductionUserDb {
    ProductionUserDb
    ::new(
        &config.user_db_directory,
        hasher,
        watcher.clone(),
    )
        .await
        .unwrap_or_else(|e|
            error_exit!("could not initialize the user DB: {e}")
        )
}

async fn make_session_storage(
    config: &CliConfig,
    watcher: ProductionFileWatcher,
) -> ProductionSessionStorage {
    ProductionSessionStorage
    ::new(
        &config.data_directory,
        watcher,
    )
        .await
        .unwrap_or_else(|e|
            error_exit!("could not initialize the session DB: {e}")
        )
}
