use std::{os::fd::RawFd, path::PathBuf};
use clap::Parser;

#[derive(Clone, Debug, Eq, Parser, PartialEq)]
#[command(version, author, about)]
pub struct CliConfig {
    #[arg(long)]
    pub config_file: Option<PathBuf>,

    #[cfg(not(debug_assertions))]
    #[arg(long, short = 'd', default_value_t = false)]
    pub no_daemonize: bool,

    #[cfg(debug_assertions)]
    #[arg(long, short = 'D', default_value_t = false)]
    pub daemonize: bool,

    #[arg(long)]
    pub public_key_file: PathBuf,

    #[arg(long)]
    pub auth_socket_fd: RawFd,

    #[arg(long)]
    pub storage_socket_fd: RawFd,
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
