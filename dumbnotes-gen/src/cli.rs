use std::path::PathBuf;
use clap::Parser;
use dumbnotes::bin_constants::DEFAULT_CONFIG_FILE;

#[derive(Clone, Debug, Eq, Parser, PartialEq)]
#[command(version, author, about)]
pub struct CliConfig {
    #[arg(long, default_value = DEFAULT_CONFIG_FILE)]
    pub config_file: PathBuf,

    #[arg(long, short = 'y', default_value_t = false)]
    pub no_repeat: bool,
    
    #[arg(group = "gen", long, default_value_t = false)]
    pub generate_jwt_key: bool,

    #[arg(group = "gen", long, default_value_t = false)]
    pub generate_pepper: bool,
}
