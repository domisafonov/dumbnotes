use std::os::fd::RawFd;
use std::path::PathBuf;
use clap::Parser;

#[derive(Clone, Debug, Eq, Parser, PartialEq)]
#[command(version, author, about = "internal auth subdaemon")]
pub struct CliConfig {
    #[arg(long)]
    pub socket_fd: RawFd,

    #[arg(long)]
    pub private_key_file: PathBuf,

    #[arg(long)]
    pub data_directory: PathBuf,
    
    #[arg(long)]
    pub user_db_directory: PathBuf,

    #[arg(long)]
    pub hasher_config: String,
}
