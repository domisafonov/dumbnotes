mod cli;
pub mod app_constants;
mod routes;
pub mod access_granter;
pub mod http;
mod app_setup;

use crate::cli::CliConfig;
use app_setup::AppSetupFairing;
use clap::{crate_name, Parser};
use dumbnotes::config::figment::FigmentExt;
use dumbnotes::error_exit;
use dumbnotes::logging::init_logging;
use figment::Figment;
use log::info;

fn main() {
    init_logging();

    info!("{} starting up", crate_name!());

    let cli_config = CliConfig::parse();
    if !cli_config.config_file.exists() {
        error_exit!(
            "configuration file at {} does not exist",
            cli_config.config_file.display()
        )
    }
    let figment = Figment::from(rocket::Config::default())
        .setup_app_config(cli_config.config_file);

    let result = rocket::execute(
        rocket
            ::custom(figment)
            .attach(AppSetupFairing::new())
            .launch()
    );
    if let Err(e) = result {
        error_exit!("failed to launch rocket: {}", e);
    }
}
