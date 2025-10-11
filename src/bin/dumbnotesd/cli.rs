use std::path::PathBuf;
use clap::Parser;
use dumbnotes::app_constants::DEFAULT_CONFIG_FILE;

#[derive(Debug, Parser)]
#[command(version, author, about)]
pub struct CliConfig {
    #[arg(long, default_value = DEFAULT_CONFIG_FILE)]
    pub config_file: PathBuf,
}
