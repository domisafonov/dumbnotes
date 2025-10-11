use std::io;
use std::process::exit;
use clap::Parser;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rpassword::prompt_password;
use dumbnotes::hasher::{Hasher, ProductionHasher};
use dumbnotes::rng::SyncRng;
use crate::cli::CliConfig;

mod cli;

// TODO: print the errors prettier
fn main() -> Result<(), io::Error> {
    let cli_config = CliConfig::parse();

    if !cli_config.config_file.exists() {
        panic!(
            "Configuration file at {} does not exist",
            cli_config.config_file.display()
        )
    }

    let hasher = ProductionHasher::new(
        SyncRng::new(StdRng::from_os_rng())
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

    println!("{}", hasher.generate_hash(&read_value));

    Ok(())
}
