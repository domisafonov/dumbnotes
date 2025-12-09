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
use dumbnotes::logging::init_daemon_logging;
#[cfg(target_os = "openbsd")] use dumbnotes::sandbox::pledge::pledge_init;
use figment::Figment;
use log::info;
use dumbnotes::nix::is_root;
use dumbnotes::nix::set_umask;
use dumbnotes::sandbox::daemonize::daemonize;

fn main() {
    #[cfg(target_os = "openbsd")] pledge_init();
    set_umask();

    let cli_config = CliConfig::parse();

    if cli_config.is_daemonizing() {
        unsafe { daemonize() }
    }

    init_daemon_logging(
        cli_config.is_daemonizing(),
    );

    info!("{} starting up", crate_name!());

    if cli_config.is_daemonizing() && !is_root() {
        error_exit!("cannot be daemonizing from a non-root user")
    }

    if !cli_config.config_file.exists() {
        error_exit!(
            "configuration file at {} does not exist",
            cli_config.config_file.display()
        )
    }
    let figment = Figment
        ::from(
            rocket::Config {
                cli_colors: !cli_config.is_daemonizing(),
                .. Default::default()
            }
        )
        .setup_app_config(&cli_config.config_file);

    let result = rocket::execute(
        rocket
            ::custom(figment)
            .attach(AppSetupFairing::new(cli_config.is_daemonizing()))
            .launch()
    );
    if let Err(e) = result {
        error_exit!("failed to launch rocket: {}", e);
    }
}
