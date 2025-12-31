mod cli;
pub mod app_constants;
mod routes;
pub mod access_granter;
pub mod http;
mod app_setup;
mod execute;

use crate::cli::CliConfig;
use app_setup::AppSetupFairing;
use clap::{crate_name, Parser};
use dumbnotes::config::read::{read_app_config, ReadConfig};
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
        unsafe { daemonize(cli_config.is_not_forking().into()) }
    }

    let authd_path = dumbnotes::ipc::exec::get_authd_executable_path()
        .unwrap_or_else(|e|
            error_exit!("failed to get authd executable path: {e}")
        );

    init_daemon_logging(
        cli_config.is_daemonizing().into(),
    );

    info!("{} starting up", crate_name!());

    let is_root = is_root();
    if !cli_config.is_daemonizing() && is_root {
        error_exit!("daemonizing is required when launching from root")
    }
    if cli_config.is_daemonizing() && !is_root {
        error_exit!("cannot be daemonizing from a non-root user")
    }

    if !cli_config.config_file.exists() {
        error_exit!(
            "configuration file at {} does not exist",
            cli_config.config_file.display()
        )
    }
    let ReadConfig {
        app_config,
        rocket_figment,
    } = read_app_config(
        &cli_config.config_file,
        Figment::from(
            rocket::Config {
                cli_colors: !cli_config.is_daemonizing(),
                .. Default::default()
            }
        ),
    )
        .unwrap_or_else(|e| 
            error_exit!("failed to read app configuration: {e}")
        );

    let result = execute::execute(
        rocket_figment,
        |fig| rocket
            ::custom(fig)
            .attach(
                AppSetupFairing::new(
                    app_config,
                    cli_config.is_daemonizing().into(),
                    authd_path,
                )
            )
            .launch()
    );
    if let Err(e) = result {
        error_exit!("failed to launch rocket: {}", e);
    }
}
