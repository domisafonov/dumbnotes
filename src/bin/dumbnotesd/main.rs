mod cli;
pub mod app_constants;
mod session_storage;
mod errors;

use crate::cli::CliConfig;
use clap::Parser;
use dumbnotes::config::app_config::AppConfig;
use dumbnotes::hasher::{ProductionHasher, ProductionHasherConfig};
use dumbnotes::rng::SyncRng;
use dumbnotes::storage::NoteStorage;
use dumbnotes::user_db::{ProductionUserDb, UserDb};
use rand::rngs::StdRng;
use rand::SeedableRng;
use rocket::figment::Figment;
use rocket::{launch, Build, Rocket};
use dumbnotes::config::figment::FigmentExt;

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

    let rng = SyncRng::new(StdRng::from_os_rng());

    let storage: NoteStorage = NoteStorage::new(
        &config,
        rng.clone(),
    ).await
        .unwrap_or_else(|e| {
            eprintln!("error: {e}");
            panic!("Initialization error");
        });

    let hasher_config = config.hasher_config.clone().try_into().unwrap_or_else(|e| {
        panic!("error: {e}");
    });
    let hasher = ProductionHasher::new(
        ProductionHasherConfig::new(hasher_config),
        rng,
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

    rocket::custom(figment)
        .manage(storage)
        .manage(config)
        .manage(user_db)
}
