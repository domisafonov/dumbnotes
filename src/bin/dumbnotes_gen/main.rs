use crate::cli::CliConfig;
use clap::Parser;
use dumbnotes::config::app_config::AppConfig;
use dumbnotes::config::figment::FigmentExt;
use dumbnotes::hasher::{Hasher, ProductionHasher, ProductionHasherConfig};
use dumbnotes::hmac_key_generator::make_hmac_key;
use figment::Figment;
use rand::rngs::OsRng;
use rpassword::prompt_password;
use std::process::exit;
use log::{error, info, warn};

mod cli;
mod config;

fn main() {
    env_logger::init();

    let cli_config = CliConfig::parse();

    if !cli_config.config_file.exists() {
        error!(
            "configuration file at {} does not exist",
            cli_config.config_file.display()
        );
        exit(1)
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
    
    if cli_config.generate_hmac_key {
        generate_hmac_key(app_config)
    } else {
        generate_hash(cli_config, app_config)
    }
}

fn generate_hash(
    cli_config: CliConfig,
    app_config: AppConfig,
) {
    let hasher_config = app_config.hasher_config.try_into()
        .unwrap_or_else(|e| {
            error!("hasher config is invalid: {}", e);
            exit(1)
        });
    let hasher = ProductionHasher::new(
        ProductionHasherConfig {
            argon2_params: hasher_config,
        },
    );

    let read_value = prompt_password("Enter the password: ")
        .unwrap_or_else(|e| {
            error!("could not read password: {}", e);
            exit(1);
        });
    if read_value.is_empty() {
        error!("entered password is empty");
        exit(1);
    }

    if !cli_config.no_repeat {
        let confirmation_value = prompt_password("Repeat the password: ")
            .unwrap_or_else(|e| {
                error!("could not read password: {}", e);
                exit(1);
            });
        if confirmation_value != read_value {
            error!("the passwords do not match");
            exit(1);
        }
    }

    if read_value.trim() != read_value {
        warn!("the password has leading or trailing whitespace characters");
    }

    let hash = hasher.generate_hash(&read_value)
        .unwrap_or_else(|e| {
            error!("could not generate hash: {}", e);
            exit(1);
        });
    println!("{}", hash);
}

fn generate_hmac_key(
    app_config: AppConfig,
) {
    make_hmac_key(&app_config, &mut OsRng)
        .unwrap_or_else(|e| {
            error!("could not generate a hmac key: {e}");
            exit(1);
        });
}
