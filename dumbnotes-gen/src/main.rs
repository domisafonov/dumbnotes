use crate::cli::CliConfig;
use clap::Parser;
use dumbnotes::config::app_config::AppConfig;
use dumbnotes::config::read::{read_app_config, ReadConfig};
use dumbnotes::error_exit;
use dumbnotes::hasher::{Hasher, ProductionHasher, ProductionHasherConfig};
#[cfg(target_os = "openbsd")] use dumbnotes::sandbox::pledge::{pledge_gen_init, pledge_gen_key, pledge_gen_hash};
#[cfg(target_os = "openbsd")] use dumbnotes::sandbox::unveil::{Permissions, unveil, seal_unveil};
use figment::Figment;
use jwt_key_generator::make_jwt_key;
use log::warn;
use rpassword::prompt_password;
use dumbnotes::nix::set_umask;
use crate::pepper_generator::make_pepper;

mod cli;
mod config;
pub mod jwt_key_generator;
mod pepper_generator;
mod file_write;

fn main() {
    #[cfg(target_os = "openbsd")] pledge_gen_init();
    set_umask();

    env_logger::init();

    let cli_config = CliConfig::parse();

    #[cfg(target_os = "openbsd")] unveil(
        &cli_config.config_file,
        Permissions::R,
    );

    if !cli_config.config_file.exists() {
        error_exit!(
            "configuration file at {} does not exist",
            cli_config.config_file.display()
        )
    }

    let ReadConfig {
        app_config,
        ..
    } = read_app_config(&cli_config.config_file, Figment::new())
        .unwrap_or_else(|e| {
            error_exit!("finishing due to a configuration error: {e}");
        });

    if cli_config.generate_jwt_key {
        generate_jwt_key(app_config)
    } else if cli_config.generate_pepper {
        generate_pepper(app_config)
    } else {
        generate_hash(cli_config, app_config)
    }
}

fn generate_jwt_key(
    app_config: AppConfig,
) {
    #[cfg(target_os = "openbsd")] {
        unveil(
            &app_config.jwt_private_key,
            Permissions::C | Permissions::W,
        );
        unveil(
            &app_config.jwt_public_key,
            Permissions::C | Permissions::W,
        );
        seal_unveil();
        pledge_gen_key();
    }

    make_jwt_key(
        &app_config.jwt_private_key,
        &app_config.jwt_public_key,
        app_config.authd_user_group.as_deref(),
    )
        .unwrap_or_else(|e| error_exit!("could not generate a jwt key: {e}"));
}

fn generate_pepper(
    app_config: AppConfig,
) {
    #[cfg(target_os = "openbsd")] {
        unveil(
            &app_config.hasher_config.pepper_path,
            Permissions::C | Permissions::W,
        );
        seal_unveil();
        pledge_gen_key();
    }
    
    make_pepper(
        &app_config.hasher_config.pepper_path,
        app_config.authd_user_group.as_deref(),
    )
        .unwrap_or_else(|e| error_exit!("could not generate pepper: {e}"));
}

fn generate_hash(
    cli_config: CliConfig,
    app_config: AppConfig,
) {
    #[cfg(target_os = "openbsd")] {
        unveil(
            &app_config.hasher_config.pepper_path,
            Permissions::R,
        );
        seal_unveil();
        pledge_gen_hash();
    }

    let hasher_config = app_config.hasher_config.make_params()
        .unwrap_or_else(|e| error_exit!("hasher config is invalid: {}", e));
    let hasher = ProductionHasher
        ::new(
            ProductionHasherConfig {
                argon2_params: hasher_config,
                pepper: app_config.hasher_config.pepper_path,
            },
        )
        .unwrap_or_else(|e|
            error_exit!("invalid hasher configuration: {e}")
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
