use std::{os::fd::RawFd, path::PathBuf};

use clap::Parser;

#[derive(Clone, Debug, Eq, Parser, PartialEq)]
#[command(version, author, about = "internal storage daemon")]
pub struct CliConfig {
    #[cfg(not(debug_assertions))]
    #[arg(long, short = 'd', default_value_t = false)]
    pub no_daemonize: bool,

    #[cfg(debug_assertions)]
    #[arg(long, short = 'D', default_value_t = false)]
    pub daemonize: bool,

    #[arg(long, required = true, value_delimiter = ',')]
    pub socket_fds: Vec<RawFd>,

    #[arg(long)]
    pub public_key_file: PathBuf,

    #[arg(long)]
    pub data_directory: PathBuf,

    #[arg(long)]
    pub max_note_len: u64,

    #[arg(long)]
    pub max_note_name_len: u64,
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
