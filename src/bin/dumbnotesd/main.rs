mod cli;

use clap::Parser;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rocket::figment::Figment;
use rocket::figment::providers::{Env, Format, Serialized, Toml};
use rocket::{launch, Build, Rocket};
use dumbnotes::config::AppConfig;
use dumbnotes::rng::SyncRng;
use dumbnotes::storage::NoteStorage;
use dumbnotes::user_db::{ProductionUserDb, UserDb};
use dumbnotes::app_constants::APP_CONFIG_ENV_PREFIX;
use crate::cli::CliConfig;

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

    // TODO: panic if unknown keys are in the config file
    let figment = Figment::from(rocket::Config::default())
        .merge(Serialized::defaults(AppConfig::default()))
        .merge(Toml::file_exact(cli_config.config_file))
        .merge(Env::prefixed(APP_CONFIG_ENV_PREFIX).global());

    let config: AppConfig = figment.extract::<AppConfig>()
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

    let user_db: Box<dyn UserDb> = Box::new(
        ProductionUserDb::new(
            &config,
            rng
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
