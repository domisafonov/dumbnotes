pub mod app_constants;
mod cli;

use clap::Parser;
use rocket::fairing::AdHoc;
use rocket::figment::Figment;
use rocket::figment::providers::{Env, Format, Serialized, Toml};
use rocket::{launch, Build, Rocket};
use dumbnotes::config::AppConfig;
use crate::app_constants::APP_CONFIG_ENV_PREFIX;
use crate::cli::CliConfig;

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

    rocket::custom(figment)
        .attach(AdHoc::config::<AppConfig>())
}
