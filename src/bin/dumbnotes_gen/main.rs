use crate::cli::CliConfig;
use clap::Parser;
use dumbnotes::config::app_config::AppConfig;
use dumbnotes::config::figment::FigmentExt;
use dumbnotes::hasher::{Hasher, ProductionHasher, ProductionHasherConfig};
use dumbnotes::hmac_key_generator::make_hmac_key;
use figment::Figment;
use rand::rngs::OsRng;
use rpassword::prompt_password;
use std::error::Error;
use std::process::exit;

mod cli;
mod config;

// TODO: print the errors prettier
fn main() -> Result<(), Box<dyn Error>> {
    let cli_config = CliConfig::parse();

    if !cli_config.config_file.exists() {
        panic!(
            "Configuration file at {} does not exist",
            cli_config.config_file.display()
        )
    }

    let app_config: AppConfig = Figment::new()
        .setup_app_config(&cli_config.config_file)
        .extract()
        .unwrap_or_else(|e| {
            for e in e {
                eprintln!("error: {e}");
            }
            panic!("Configuration error");
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
) -> Result<(), Box<dyn Error>> {
    let hasher = ProductionHasher::new(
        ProductionHasherConfig {
            argon2_params: app_config.hasher_config.try_into()?,
        },
    );

    let read_value = prompt_password("Enter the password: ")?;
    if read_value.is_empty() {
        eprintln!("error: entered password is empty");
        exit(1);
    }

    if !cli_config.no_repeat {
        let confirmation_value = prompt_password("Repeat the password: ")?;
        if confirmation_value != read_value {
            eprintln!("error: the passwords do not match");
            exit(1);
        }
    }

    if read_value.trim() != read_value {
        eprintln!("warning: the password has leading or trailing whitespace characters");
    }

    println!("{}", hasher.generate_hash(&read_value)?);

    Ok(())
}

fn generate_hmac_key(
    app_config: AppConfig,
) -> Result<(), Box<dyn Error>> {
    make_hmac_key(&app_config, &mut OsRng)?;
    Ok(())
}