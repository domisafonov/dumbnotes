mod cli;
pub mod app_constants;
mod session_storage;
pub mod user_db;
mod routes;
mod access_token;
pub mod access_granter;
pub mod http;
pub mod file_watcher;

use crate::access_granter::AccessGranter;
use crate::access_token::{AccessTokenDecoder, AccessTokenGenerator};
use crate::cli::CliConfig;
use crate::file_watcher::ProductionFileWatcher;
use crate::routes::{ApiRocketBuildExt, WebRocketBuildExt};
use crate::session_storage::ProductionSessionStorage;
use crate::user_db::{ProductionUserDb, UserDb};
use clap::{crate_name, Parser};
use dumbnotes::config::app_config::AppConfig;
use dumbnotes::config::figment::FigmentExt;
use dumbnotes::hasher::{ProductionHasher, ProductionHasherConfig};
use dumbnotes::storage::NoteStorage;
use figment::Figment;
use josekit::jwk::Jwk;
use rocket::{launch, Build, Rocket};
use std::error::Error;
use std::path::Path;
use std::process::exit;
use log::{error, info};

#[launch]
async fn rocket() -> Rocket<Build> {
    init_logging();

    info!("{} starting up", crate_name!());

    let cli_config = CliConfig::parse();

    if !cli_config.config_file.exists() {
        error!(
            "configuration file at {} does not exist",
            cli_config.config_file.display()
        );
        exit(1)
    }

    let figment = Figment::from(rocket::Config::default())
        .setup_app_config(cli_config.config_file);
    let config: AppConfig = figment.extract()
        .unwrap_or_else(|e| {
            for e in e {
                error!("{e}");
            }
            info!("finishing due to a config parse error");
            exit(1)
        });

    let storage: NoteStorage = NoteStorage::new(&config)
        .await
        .unwrap_or_else(|e| {
            error!("note storage initialization failed: {e}");
            exit(1)
        });

    let hasher_config = config.hasher_config.clone().try_into().unwrap_or_else(|e| {
        error!("hasher config read failed: {e}");
        exit(1)
    });
    let hasher = ProductionHasher::new(
        ProductionHasherConfig::new(hasher_config),
    );

    let watcher = ProductionFileWatcher::new()
        .unwrap_or_else(|e| {
            error!("failed to create file watcher: {e}");
            exit(1)
        });

    let user_db: Box<dyn UserDb> = Box::new(
        ProductionUserDb::new(
            &config,
            hasher,
            watcher.clone(),
        ).await
            .unwrap_or_else(|e| {
                error!("could not initialize the user DB: {e}");
                exit(1)
            })
    );

    let session_storage = Box::new(
        ProductionSessionStorage
            ::new(
                &config,
                watcher,
            )
            .await
            .unwrap_or_else(|e| {
                error!("could not initialize the session DB: {e}");
                exit(1)
            })
    );

    let hmac_key = read_hmac_key(&config.hmac_key)
        .unwrap_or_else(|e| {
            error!("failed reading the hmac key: {e}");
            exit(1)
        });
    let access_token_generator = AccessTokenGenerator::from_jwk(&hmac_key)
        .unwrap_or_else(|e| {
            error!("could not initialize access token generator: {e}");
            exit(1)
        });
    let access_token_decoder = AccessTokenDecoder::from_jwk(&hmac_key)
        .unwrap_or_else(|e| {
            error!("could not initialize access token decoder: {e}");
            exit(1)
        });

    let access_granter = AccessGranter::new(
        session_storage,
        user_db,
        access_token_generator,
        access_token_decoder,
    );

    rocket::custom(figment)
        .manage(storage)
        .manage(config)
        .manage(access_granter)
        .install_dumbnotes_api()
        .install_dumbnotes_web()
}

fn read_hmac_key(path: &Path) -> Result<Jwk, Box<dyn Error>> {
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

#[cfg(debug_assertions)]
fn init_logging() {
    env_logger::init()
}

#[cfg(not(debug_assertions))]
fn init_logging() {
    use syslog::BasicLogger;

    log
    ::set_boxed_logger(
        Box::new(
            BasicLogger::new(
                syslog::unix(
                    // for some reason, only 3164 has log crate
                    // integration at the moment
                    syslog::Formatter3164::default(),
                ).expect("syslog initialization failed")
            )
        )
    )
        .map(|()| log::set_max_level(log::STATIC_MAX_LEVEL))
        .expect("syslog initialization failed");
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
        error!("{message}");
        exit(1)
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
