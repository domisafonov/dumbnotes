use std::path::PathBuf;
use clap::Parser;
use dumbnotes::bin_constants::DEFAULT_CONFIG_FILE;

#[derive(Clone, Debug, Eq, Parser, PartialEq)]
#[command(version, author, about)]
pub struct CliConfig {
    #[arg(long, default_value = DEFAULT_CONFIG_FILE)]
    pub config_file: PathBuf,

    #[cfg(not(debug_assertions))]
    #[arg(long, short = 'd', default_value_t = false)]
    pub no_daemonize: bool,

    #[cfg(debug_assertions)]
    #[arg(long, short = 'D', default_value_t = false)]
    pub daemonize: bool,

    #[cfg(debug_assertions)]
    #[arg(long, default_value_t = false)]
    pub no_fork: bool,
}

impl CliConfig {
    #[cfg(not(debug_assertions))]
    pub fn is_daemonizing(&self) -> bool {
        !self.no_daemonize
    }

    #[cfg(debug_assertions)]
    pub fn is_daemonizing(&self) -> bool {
        self.daemonize
    }

    #[cfg(not(debug_assertions))]
    pub fn is_not_forking(&self) -> bool {
        false
    }

    #[cfg(debug_assertions)]
    pub fn is_not_forking(&self) -> bool {
        self.no_fork
    }
}
