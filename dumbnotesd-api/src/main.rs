use std::path::PathBuf;

use clap::{Parser, crate_name};
use dumbnotes::{logging::init_daemon_logging};
#[cfg(target_os = "openbsd")] use dumbnotes::sandbox::pledge::pledge_apid_init;
use dumbnotesd_api::{app_constants::DEFAULT_API_PORT, app_setup::AppSetupFairing, cli::CliConfig};
use figment::{Figment, providers::Format};
use log::info;
use unix::set_umask;
use util::error_exit;

fn main() {
    #[cfg(target_os = "openbsd")] pledge_apid_init();
    set_umask();

    let cli_config = CliConfig::parse();

    init_daemon_logging(cli_config.is_daemonizing().into());

    info!("{} starting up", crate_name!());

    let rocket_defaults = Figment::from(
        rocket::Config {
            cli_colors: !cli_config.is_daemonizing(),
            port: DEFAULT_API_PORT,
            .. Default::default()
        }
    );
    let rocket_figment = match cli_config.config_file {
        Some(config) if config.exists() => rocket_defaults
            .merge(figment::providers::Toml::file_exact(config)),
        Some(config) => error_exit!(
            "configuration file at {} does not exist",
            config.display(),
        ),
        None => rocket_defaults,
    };

    let result = rocket_execute::execute(
        rocket_figment,
        |fig| {
            let temp_dir = fig.extract_inner::<PathBuf>("temp_dir")
                .unwrap_or_else(|_| std::env::temp_dir());
            rocket
                ::custom(fig)
                .attach(
                    AppSetupFairing::new(
                        cli_config.public_key_file,
                        cli_config.auth_socket_fd,
                        cli_config.storage_socket_fd,
                        temp_dir,
                    )
                )
                .launch()
        }
    );
    if let Err(e) = result {
        error_exit!("failed to launch rocket: {}", e);
    }
}
