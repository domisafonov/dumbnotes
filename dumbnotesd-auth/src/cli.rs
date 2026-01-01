use std::os::fd::RawFd;
use std::path::PathBuf;
use clap::Parser;

#[derive(Clone, Debug, Eq, Parser, PartialEq)]
#[command(version, author, about = "internal auth subdaemon")]
pub struct CliConfig {
    #[cfg(not(debug_assertions))]
    #[arg(long, short = 'd', default_value_t = false)]
    pub no_daemonize: bool,

    #[cfg(debug_assertions)]
    #[arg(long, short = 'D', default_value_t = false)]
    pub daemonize: bool,

    #[arg(long)]
    pub socket_fd: RawFd,

    #[arg(long)]
    pub private_key_file: PathBuf,

    #[arg(long)]
    pub data_directory: PathBuf,
    
    #[arg(long)]
    pub user_db_path: PathBuf,

    #[arg(long)]
    pub hasher_config: String,
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
}
