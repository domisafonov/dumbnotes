mod cli;
mod eventloop;
mod processors;
pub mod session_storage;
mod app_constants;
pub mod user_db;
pub mod access_token_generator;
pub mod file_watcher;

use crate::access_token_generator::AccessTokenGenerator;
use crate::app_constants::SHUTDOWN_TIMEOUT;
use crate::cli::CliConfig;
use clap::{crate_name, Parser};
use dumbnotes::config::hasher_config::ProductionHasherConfigData;
use dumbnotes::bin_constants::IPC_MESSAGE_MAX_SIZE;
use dumbnotes::ipc::launch_event_loops::launch_event_loops;
use util::error_exit;
use dumbnotes::hasher::{ProductionHasher, ProductionHasherConfig};
use dumbnotes::logging::init_daemon_logging;
#[cfg(target_os = "openbsd")] use dumbnotes::sandbox::pledge::{pledge_authd_init, pledge_authd_normal};
use file_watcher::ProductionFileWatcher;
use josekit::jwk::Jwk;
use log::info;
use session_storage::ProductionSessionStorage;
use std::error::Error;
use std::path::Path;
use unix::{check_secret_file_ro_access, set_umask};
use user_db::ProductionUserDb;

#[tokio::main]
async fn main() {
    #[cfg(target_os = "openbsd")] pledge_authd_init();
    set_umask();

    let config = CliConfig::parse();
    let hasher_config = parse_hasher_config(&config);
    #[cfg(target_os = "openbsd")] {
        use dumbnotes::sandbox::unveil::{Permissions, unveil, seal_unveil};

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
            &hasher_config.pepper_path,
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

    launch_event_loops(
        crate_name!(),
        config.socket_fds.clone(),
        async move || {
            let watcher = ProductionFileWatcher::new()
                .unwrap_or_else(|e| error_exit!("failed to create file watcher: {e}"));
            eventloop::State {
                token_generator: make_token_generator(&config),
                user_db: make_user_db(
                    &config,
                    make_hasher(&hasher_config),
                    watcher.clone(),
                ).await,
                session_storage: make_session_storage(&config, watcher).await,
            }
        },
        |state, stream, write_socket|
            eventloop::process_commands(
                state,
                stream,
                write_socket,
            ),
        IPC_MESSAGE_MAX_SIZE,
        || { #[cfg(target_os = "openbsd")] pledge_authd_normal() },
        SHUTDOWN_TIMEOUT,
    ).await;
}

fn parse_hasher_config(config: &CliConfig) -> ProductionHasherConfigData {
    serde_json
    ::from_str(&config.hasher_config)
        .unwrap_or_else(|e|
            error_exit!("hasher config is invalid: {e}")
        )
}

fn make_hasher(
    hasher_config: &ProductionHasherConfigData,
) -> ProductionHasher {
    let params: argon2::Params = hasher_config.make_params().unwrap_or_else(|e| {
        error_exit!("hasher config read failed: {e}")
    });
    ProductionHasher
        ::new(
            ProductionHasherConfig
                ::new(
                    params,
                    hasher_config.pepper_path.to_owned(),
                )
        )
        .unwrap_or_else(|e|
            error_exit!("failed to initialize the hasher {e}")
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
