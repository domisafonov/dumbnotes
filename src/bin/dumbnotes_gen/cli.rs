use std::path::PathBuf;
use clap::Parser;
use dumbnotes::app_constants::DEFAULT_CONFIG_FILE;
use dumbnotes::cli::ProductionHasherConfig;

#[derive(Clone, Debug, Eq, Parser, PartialEq)]
#[command(version, author, about)]
pub struct CliConfig {
    #[arg(long, default_value = DEFAULT_CONFIG_FILE)]
    pub config_file: PathBuf,

    #[arg(long, short = 'y', default_value_t = false)]
    pub no_repeat: bool,

    #[command(flatten)]
    pub hasher_config: ProductionHasherConfig,
}
