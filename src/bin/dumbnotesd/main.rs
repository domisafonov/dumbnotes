mod cli;
pub mod app_constants;
mod session_storage;
pub mod user_db;
mod routes;
mod access_token;

use crate::app_constants::{API_PREFIX, WEB_PREFIX};
use crate::cli::CliConfig;
use crate::routes::{api_routes, web_routes};
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
use crate::access_token::{AccessTokenDecoder, AccessTokenGenerator};

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

    let session_storage = ProductionSessionStorage::new(&config)
        .await
        .unwrap_or_else(|e| {
            eprintln!("error: {e}");
            panic!("Initialization error");
        });

    // TODO: check file permissions on start
    let hmac_key = std::fs::read(&config.hmac_key)
        .map(Jwk::from_bytes)
        .unwrap_or_else(|e| {
            eprintln!("error: {e}");
            panic!("Initialization error");
        })
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

    rocket::custom(figment)
        .manage(storage)
        .manage(config)
        .manage(user_db)
        .mount(API_PREFIX, api_routes())
        .mount(WEB_PREFIX, web_routes())
}
