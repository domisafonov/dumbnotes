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
#[cfg(target_os = "openbsd")] use dumbnotes::sandbox::pledge::{pledge_authd_init, pledge_authd_normal};
#[cfg(target_os = "openbsd")] use dumbnotes::sandbox::unveil::{Permissions, unveil, seal_unveil};
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
use dumbnotes::nix::check_secret_file_ro_access;
use dumbnotes::nix::set_umask;
use user_db::ProductionUserDb;

#[tokio::main]
async fn main() {
    #[cfg(target_os = "openbsd")] pledge_authd_init();
    set_umask();

    let config = CliConfig::parse();
    #[cfg(target_os = "openbsd")] {
        unveil(
            &std::path::PathBuf::from("/dev/log"),
            Permissions::W,
        );
        unveil(
            &config.private_key_file,
            Permissions::R,
        );
        unveil(
            &config.user_db_path,
            Permissions::R,
        );
        unveil(
            &ProductionSessionStorage::get_storage_path(&config.data_directory),
            Permissions::R | Permissions::W | Permissions::C,
        );
        seal_unveil();
    }

    init_daemon_logging(config.is_daemonizing().into());

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
    check_secret_file_ro_access(path)?;
    Ok(
        Jwk::from_bytes(
            std::fs::read(path)?
        )?
    )
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
