mod app_constants;
mod cli;
mod eventloop;
mod processors;
mod storage;
mod util;

use std::path::Path;

use clap::{Parser, crate_name};
use dumbnotes::{bin_constants::IPC_STORAGE_MESSAGE_MAX_SIZE, ipc::{launch_event_loops::launch_event_loops}, logging::init_daemon_logging};
#[cfg(target_os = "openbsd")] use dumbnotes::sandbox::pledge::{pledge_storage_init, pledge_storage_normal};
use log::info;
use storage::{errors::*, NoteStorage};
use unix::set_umask;
use ::util::error_exit;

use crate::{app_constants::SHUTDOWN_TIMEOUT, cli::CliConfig};

async fn async_main() -> i32 {
    #[cfg(target_os = "openbsd")] pledge_storage_init();
    set_umask();

    let config = CliConfig::parse();
    #[cfg(target_os = "openbsd")] {
        use dumbnotes::sandbox::unveil::{Permissions, unveil, seal_unveil};

        unveil(
            &std::path::PathBuf::from("/dev/log"),
            Permissions::W,
        );
        unveil(
            &config.public_key_file,
            Permissions::R,
        );
        unveil(
            &NoteStorage::get_notes_dir(&config.data_directory),
            Permissions::R | Permissions::W | Permissions::C,
        );
        seal_unveil();
    }

    init_daemon_logging(config.is_daemonizing().into());

    info!("{} starting up", crate_name!());

    launch_event_loops(
        crate_name!(),
        config.socket_fds,
        async move || {
            eventloop::State {
                note_storage: make_note_storage(
                    &config.data_directory,
                    config.max_note_len,
                    config.max_note_name_len,
                ).await,
            }
        },
        eventloop::process_commands,
        IPC_STORAGE_MESSAGE_MAX_SIZE,
        || { #[cfg(target_os = "openbsd")] pledge_storage_normal() },
        SHUTDOWN_TIMEOUT,
    ).await
}

async fn make_note_storage(
    data_directory: impl AsRef<Path>,
    max_note_len: u64,
    max_note_name_len: u64,
) -> NoteStorage {
    NoteStorage
        ::new(
            &data_directory,
            max_note_len,
            max_note_name_len,
        )
        .await
        .unwrap_or_else(|e|
            error_exit!(
                "failed to initialize note storage as {}: {e}",
                data_directory.as_ref().display(),
            )
        )
}

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let exit_code = runtime.block_on(async_main());
    runtime.shutdown_background();
    std::process::exit(exit_code);
}
