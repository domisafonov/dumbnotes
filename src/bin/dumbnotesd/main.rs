mod cli;
pub mod app_constants;
mod session_storage;
pub mod user_db;
mod routes;
mod access_token;
pub mod access_granter;
pub mod http;

use crate::access_granter::AccessGranter;
use crate::access_token::{AccessTokenDecoder, AccessTokenGenerator};
use crate::app_constants::{API_PREFIX, WEB_PREFIX};
use crate::cli::CliConfig;
use crate::routes::{api_catchers, api_routes, web_routes};
use crate::session_storage::ProductionSessionStorage;
use crate::user_db::{ProductionUserDb, UserDb};
use clap::Parser;
use dumbnotes::config::app_config::AppConfig;
use dumbnotes::config::figment::FigmentExt;
use dumbnotes::hasher::{ProductionHasher, ProductionHasherConfig};
use dumbnotes::storage::NoteStorage;
use figment::Figment;
use josekit::jwk::Jwk;
use rocket::{launch, Build, Rocket};
use std::error::Error;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

// TODO: print the errors prettier
#[launch]
async fn rocket() -> Rocket<Build> {
    let cli_config = CliConfig::parse();

    if !cli_config.config_file.exists() {
        panic!(
            "Configuration file at {} does not exist",
            cli_config.config_file.display()
        )
    }

    let figment = Figment::from(rocket::Config::default())
        .setup_app_config(cli_config.config_file);
    let config: AppConfig = figment.extract()
        .unwrap_or_else(|e| {
            for e in e {
                eprintln!("error: {e}");
            }
            panic!("Configuration error");
        });

    let storage: NoteStorage = NoteStorage::new(&config)
        .await
        .unwrap_or_else(|e| {
            eprintln!("error: {e}");
            panic!("Initialization error");
        });

    let hasher_config = config.hasher_config.clone().try_into().unwrap_or_else(|e| {
        panic!("error: {e}");
    });
    let hasher = ProductionHasher::new(
        ProductionHasherConfig::new(hasher_config),
    );

    let user_db: Box<dyn UserDb> = Box::new(
        ProductionUserDb::new(
            &config,
            hasher,
        ).await
            .unwrap_or_else(|e| {
                eprintln!("error: {e}");
                panic!("Initialization error");
            })
    );

    let session_storage = Box::new(
        ProductionSessionStorage::new(&config)
            .await
            .unwrap_or_else(|e| {
                eprintln!("error: {e}");
                panic!("Initialization error");
            })
    );

    let hmac_key = read_hmac_key(&config.hmac_key)
        .unwrap_or_else(|e| {
            eprintln!("error: {e}");
            panic!("Initialization error");
        });
    let access_token_generator = AccessTokenGenerator::from_jwk(&hmac_key)
        .unwrap_or_else(|e| {
            eprintln!("error: {e}");
            panic!("Initialization error");
        });
    let access_token_decoder = AccessTokenDecoder::from_jwk(&hmac_key)
        .unwrap_or_else(|e| {
            eprintln!("error: {e}");
            panic!("Initialization error");
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
        .mount(API_PREFIX, api_routes())
        .register(API_PREFIX, api_catchers())
        .mount(WEB_PREFIX, web_routes())
}

fn read_hmac_key(path: &Path) -> Result<Jwk, Box<dyn Error>> {
    test_permissions(
        path,
        |p| p == 0o600 || p == 0o400,
        &format!(
            "error: {} must be owned by root and have mode of 600 or 400",
            path.to_string_lossy(),
        )
    )?;
    test_permissions(
        path.parent().expect("path has no parent"),
        |p| p & 0o022 == 0,
        &format!(
            "error: {} must be owned by root and not be writeable by group or other",
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
    let metadata = std::fs::metadata(path)?;
    let permissions = metadata.permissions().mode() & 0o777;
    if metadata.uid() != 0 || !is_valid(permissions) {
        eprintln!("{message}");
        panic!("Initialization error");
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
