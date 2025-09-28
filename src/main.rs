pub mod app_constants;
mod cli;

use clap::Parser;
use rocket::figment::Figment;
use rocket::figment::providers::{Env, Format, Serialized, Toml};
use rocket::{launch, Build, Rocket};
use dumbnotes::config::AppConfig;
use dumbnotes::storage::NoteStorage;
use crate::app_constants::APP_CONFIG_ENV_PREFIX;
use crate::cli::CliConfig;

// TODO: error messages
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
        .merge(Serialized::defaults(AppConfig::default()))
        .merge(Toml::file_exact(cli_config.config_file))
        .merge(Env::prefixed(APP_CONFIG_ENV_PREFIX).global());

    let config: AppConfig = figment.extract::<AppConfig>()
        .expect("Can't parse configuration");

    let storage: NoteStorage = NoteStorage::new(&config).await
        .expect("Can't create note storage");

    rocket::custom(figment)
        .manage(storage)
        .manage(config)
}
