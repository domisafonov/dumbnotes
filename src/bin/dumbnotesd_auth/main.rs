mod cli;
mod eventloop;
mod processors;
pub mod session_storage;
mod app_constants;
pub mod user_db;
pub mod access_token_generator;
pub mod file_watcher;

use crate::access_token_generator::AccessTokenGenerator;
use crate::cli::CliConfig;
use clap::{crate_name, Parser};
use dumbnotes::config::hasher_config::ProductionHasherConfigData;
use dumbnotes::error_exit;
use dumbnotes::hasher::{ProductionHasher, ProductionHasherConfig};
use dumbnotes::ipc::auth::message_stream;
use dumbnotes::logging::init_daemon_logging;
#[cfg(target_os = "openbsd")] use dumbnotes::pledge::{pledge_authd_init, pledge_authd_normal};
use file_watcher::ProductionFileWatcher;
use josekit::jwk::Jwk;
use log::info;
use session_storage::ProductionSessionStorage;
use socket2::Socket;
use std::error::Error;
use std::io;
use std::os::fd::FromRawFd;
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::Path;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::UnixStream;
use user_db::ProductionUserDb;

#[tokio::main]
async fn main() {
    #[cfg(target_os = "openbsd")] pledge_authd_init();

    let config = CliConfig::parse();
    init_daemon_logging(config.is_daemonizing());

    info!("{} starting up", crate_name!());

    let (read_socket, write_socket) = make_sockets(&config);
    let watcher = ProductionFileWatcher::new()
        .unwrap_or_else(|e| error_exit!("failed to create file watcher: {e}"));
    let hasher = make_hasher(&config);

    let the_loop = eventloop::process_commands(
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
        message_stream::stream(read_socket),
        write_socket,
    );
    #[cfg(target_os = "openbsd")] pledge_authd_normal();
    let result = tokio::spawn(the_loop).await;

    if let Err(e) = result {
        error_exit!("event loop finished with error: {e}")
    }

    info!("{} terminating normally", crate_name!());
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
            StdUnixStream::from(command_socket),
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
            error_exit!("could not initialize access token access_token_generator: {e}")
        )
}

fn read_jwt_key(
    path: &Path,
) -> Result<Jwk, Box<dyn Error>> {
    test_permissions(
        path,
        |p| p == 0o600 || p == 0o400,
        &format!(
            "{} must be owned by TODO and have mode of 600 or 400",
            path.to_string_lossy(),
        )
    )?;
    test_permissions(
        path.parent().expect("path has no parent"),
        |p| p & 0o022 == 0,
        &format!(
            "{} must be owned by TODO and not be writeable by group or other",
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
    // TODO: if metadata.uid() != <configured user> || !is_valid(permissions) {
    if !is_valid(permissions) {
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
        &config.user_db_path,
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
