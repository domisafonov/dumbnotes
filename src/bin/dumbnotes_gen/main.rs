use crate::cli::CliConfig;
use clap::Parser;
use dumbnotes::config::app_config::AppConfig;
use dumbnotes::config::figment::FigmentExt;
use dumbnotes::error_exit;
use dumbnotes::hasher::{Hasher, ProductionHasher, ProductionHasherConfig};
#[cfg(target_os = "openbsd")] use dumbnotes::sandbox::pledge::{pledge_gen_init, pledge_gen_key, pledge_gen_hash};
use figment::Figment;
use jwt_key_generator::make_jwt_key;
use log::{error, info, warn};
use rpassword::prompt_password;
use std::process::exit;

mod cli;
mod config;
pub mod jwt_key_generator;

fn main() {
    #[cfg(target_os = "openbsd")] pledge_gen_init();

    env_logger::init();

    let cli_config = CliConfig::parse();

    if !cli_config.config_file.exists() {
        error_exit!(
            "configuration file at {} does not exist",
            cli_config.config_file.display()
        )
    }

    let app_config: AppConfig = Figment::new()
        .setup_app_config(&cli_config.config_file)
        .extract()
        .unwrap_or_else(|e| {
            for e in e {
                error!("{e}");
            }
            info!("finishing due to a configuration error");
            exit(1)
        });

    if cli_config.generate_jwt_key {
        generate_jwt_key(app_config)
    } else {
        generate_hash(cli_config, app_config)
    }
}

fn generate_hash(
    cli_config: CliConfig,
    app_config: AppConfig,
) {
    #[cfg(target_os = "openbsd")] pledge_gen_hash();

    let hasher_config = app_config.hasher_config.try_into()
        .unwrap_or_else(|e| error_exit!("hasher config is invalid: {}", e));
    let hasher = ProductionHasher::new(
        ProductionHasherConfig {
            argon2_params: hasher_config,
        },
    );

    let read_value = prompt_password("Enter the password: ")
        .unwrap_or_else(|e| error_exit!("could not read password: {}", e));
    if read_value.is_empty() {
        error_exit!("entered password is empty")
    }

    if !cli_config.no_repeat {
        let confirmation_value = prompt_password("Repeat the password: ")
            .unwrap_or_else(|e| error_exit!("could not read password: {}", e));
        if confirmation_value != read_value {
            error_exit!("the passwords do not match")
        }
    }

    if read_value.trim() != read_value {
        warn!("the password has leading or trailing whitespace characters");
    }

    let hash = hasher.generate_hash(&read_value)
        .unwrap_or_else(|e| error_exit!("could not generate hash: {}", e));
    println!("{}", hash);
}

fn generate_jwt_key(
    app_config: AppConfig,
) {
    #[cfg(target_os = "openbsd")] pledge_gen_key();

    make_jwt_key(&app_config.jwt_private_key, &app_config.jwt_public_key)
        .unwrap_or_else(|e| error_exit!("could not generate a jwt key: {e}"));
}
