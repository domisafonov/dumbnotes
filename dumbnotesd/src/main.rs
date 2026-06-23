use std::{ffi::{OsStr, OsString}, os::fd::{AsRawFd, OwnedFd}, process::ExitStatus};

use clap::{crate_name, Parser};
use dumbnotes::{config::{app_config::AppConfig, read::read_app_config}, ipc::socket::create_socket_pair, sandbox::user_group::{clear_supplementary_groups, set_user_and_group}};
use dumbnotesd::{app_constants::{EXTRA_SHUTDOWN_TIMEOUT, SHUTDOWN_TIMEOUT}, cli::CliConfig, exec_path::{get_apid_executable_path, get_authd_executable_path, get_storaged_executable_path, get_webd_executable_path}, kill_with_timeout::{KillWithTimeoutChildExt, SendTerm}, launch_sub::{launch_sub, launch_sub_with_sockets}};
use futures::{FutureExt, Stream, future::{join_all, select_all}};
use socket2::Socket;
use tap::Pipe;
use tokio::{process::{Child, Command}, signal::unix::{SignalKind, signal}, time::Instant};
use tokio_stream::{StreamExt, wrappers::SignalStream};
use util::{OptionStrRefExt, error_exit};
use dumbnotes::logging::init_daemon_logging;
#[cfg(target_os = "openbsd")] use dumbnotes::sandbox::pledge::pledge_init;
use log::{error, info};
use dumbnotes::sandbox::daemonize::daemonize;
use unix::{is_root, set_umask};

// FIXME: process the signals
fn main() {
    #[cfg(target_os = "openbsd")] pledge_init(); // FIXME

    set_umask();

    let cli_config = CliConfig::parse();

    if cli_config.is_daemonizing() {
        unsafe { daemonize(cli_config.is_not_forking().into()) }
        clear_supplementary_groups()
            .unwrap_or_else(|e|
                error_exit!("error clearing supplementary groups: {e}")
            );
    }

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main(cli_config))
}

async fn async_main(cli_config: CliConfig) {
    init_daemon_logging(
        cli_config.is_daemonizing().into(),
    );

    info!("{} starting up", crate_name!());

    let is_root = is_root();
    if !cli_config.is_daemonizing() && is_root {
        error_exit!("daemonizing is required when launching from root")
    }
    if cli_config.is_daemonizing() && !is_root {
        error_exit!("cannot be daemonizing from a non-root user")
    }

    if !cli_config.config_file.exists() {
        error_exit!(
            "configuration file at {} does not exist",
            cli_config.config_file.display()
        )
    }

    let app_config = read_app_config(&cli_config.config_file)
        .unwrap_or_else(|e|
            error_exit!("failed to read the config file: {e}")
        );

    if !app_config.is_api_enabled && !app_config.is_web_enabled {
        error_exit!("all network servers are disable in the configuration")
    }

    let mut shutdown_signals = intercept_singals().await;
    let mut spawns = spawn_children(&cli_config, &app_config).await;

    if let Some(ref empty_user_group) = app_config.empty_user_group {
        set_user_and_group(&empty_user_group)
            .unwrap_or_else(|e|
                error_exit!("failed to set user and group: {e}")
            )
    }

    // FIXME: chroot to /var/empty
    // FIXME: pledge
    // FIXME: unveil to nothingness

    let daemon_termination = select_all(
        spawns.daemons
            .iter_mut()
            .map(|c| c.wait().boxed())
            .collect::<Vec<_>>()
    );
    let server_termination = select_all(
        spawns.servers
            .iter_mut()
            .map(|c| c.wait().boxed())
            .collect::<Vec<_>>()
    );
    tokio::select! {
        _ = shutdown_signals.next() => info!("received a shutdown signal"),
        (_, _, _) = daemon_termination =>
            error!("a subdaemon terminated unexpectedly, shutting down"),
        (_, _, _) = server_termination =>
            error!("a subserver terminated unexpectedly, shutting down"),
    };

    let deadline = Instant::now() + SHUTDOWN_TIMEOUT;
    let shutdown_fut = shutdown(deadline, &mut spawns.servers, SendTerm::Yes);
    for res in shutdown_fut.await {
        match res {
            Err(e) => error!("failed to terminate a server child: {e}"),
            Ok(status) if !status.success() =>
                error!("a server child terminated with status {status}"),
            _ => (),
        }
    }
    tokio::time::sleep_until(deadline).await;

    let shutdown_fut = shutdown(
        deadline + EXTRA_SHUTDOWN_TIMEOUT,
        &mut spawns.daemons,
        SendTerm::No,
    );
    for res in shutdown_fut.await {
        match res {
            Err(e) => error!("failed to terminate a daemon child: {e}"),
            Ok(status) if !status.success() =>
                error!("a daemon child terminated with status {status}"),
            _ => (),
        }
    }

    info!("shutting the manager process down");
}

async fn intercept_singals() -> impl Stream<Item=()> {
    let int_signal = signal(SignalKind::interrupt())
        .unwrap_or_else(|e|
            error_exit!("failed to set up signal handlers: {e}")
        );
    let term_signal = signal(SignalKind::terminate())
        .unwrap_or_else(|e|
            error_exit!("failed to set up signal handlers: {e}")
        );
    SignalStream::new(int_signal)
        .merge(SignalStream::new(term_signal))
}

async fn spawn_children(
    cli_config: &CliConfig,
    app_config: &AppConfig,
) -> Spawns {
    fn store_pair(
        sockets: &mut Vec<Socket>,
        is_enabled: bool,
    ) -> Option<Socket> {
        let (cloexec_socket, immediate_use_socket) = create_socket_pair()
            .unwrap_or_else(|e|
                error_exit!("failed to create an IPC socket pair")
            );
        if is_enabled.into() {
            sockets.push(immediate_use_socket);
            Some(cloexec_socket)
        } else {
            None
        }
    }
    fn set_cloexec(socket: &Socket, is_cloexec: bool) {
        socket.set_cloexec(is_cloexec)
            .unwrap_or_else(|e|
                error_exit!("failed to change cloexec on an IPC socket: {e}")
            )
    }

    let authd_path = get_authd_executable_path()
        .unwrap_or_else(|e|
            error_exit!("failed to get authd executable path: {e}")
        );
    let mut sockets = Vec::new();
    let api_socket_to_auth = store_pair(&mut sockets, app_config.is_api_enabled);
    let web_socket_to_auth = store_pair(&mut sockets, app_config.is_web_enabled);
    let auth_child = launch_sub_with_sockets(
        authd_path,
        app_config.authd_user_group.as_str_ref(),
        sockets.iter().map(AsRawFd::as_raw_fd),
        cli_config.is_daemonizing().into(),
        |command: &mut Command| {
            command
                .arg(path_arg("private-key-file", &app_config.jwt_private_key))
                .arg(path_arg("data-directory", &app_config.data_directory))
                .arg(path_arg("user-db-path", &app_config.user_db))
                .arg(
                    format!(
                        "--hasher-config={}",
                        serde_json::to_string(&app_config.hasher_config)
                            .unwrap_or_else(|e|
                                error_exit!(
                                    "cannot serialize hasher config: {e}"
                                )
                            )
                    )
                );
        },
    ).await
        .unwrap_or_else(|e| error_exit!("failed to launch authd: {e}"));
    sockets.clear();

    let storaged_path = get_storaged_executable_path()
        .unwrap_or_else(|e|
            error_exit!("failed to get authd executable path: {e}")
        );
    let api_socket_to_storage = store_pair(&mut sockets, app_config.is_api_enabled);
    let web_socket_to_storage = store_pair(&mut sockets, app_config.is_web_enabled);
    let storage_child = launch_sub_with_sockets(
        storaged_path,
        app_config.storage_user_group.as_str_ref(),
        sockets.iter().map(AsRawFd::as_raw_fd),
        cli_config.is_daemonizing().into(),
        |command| {
            command
                .arg(path_arg("public-key-file", &app_config.jwt_public_key))
                .arg(path_arg("data-directory", &app_config.data_directory))
                .arg(format!("--max-note-len={}", &app_config.max_note_size))
                .arg(
                    format!(
                        "--max-note-name-len={}",
                        app_config.max_note_name_size,
                    )
                );
        },
    ).await
        .unwrap_or_else(|e| error_exit!("failed to launch storaged: {e}"));
    drop(sockets);

    let api_child = if app_config.is_api_enabled {
        let api_socket_to_auth = api_socket_to_auth.unwrap();
        let api_socket_to_storage = api_socket_to_storage.unwrap();
        set_cloexec(&api_socket_to_auth, false);
        set_cloexec(&api_socket_to_storage, false);
        let apid_path = get_apid_executable_path()
            .unwrap_or_else(|e|
                error_exit!("failed to get apid executable path: {e}")
            );
        launch_sub(
            apid_path,
            app_config.empty_user_group.as_str_ref(),
            cli_config.is_daemonizing().into(),
            |command| {
                if let Some(ref api_rocket_config) = app_config.api_rocket_config {
                    command.arg(
                        path_arg("config-file", api_rocket_config)
                    );
                }
                command
                    .arg(
                        path_arg("public-key-file",
                        &app_config.jwt_public_key,
                    ))
                    .arg(socket_arg("auth-socket-fd", &api_socket_to_auth))
                    .arg(
                        socket_arg("storage-socket-fd", &api_socket_to_storage)
                    );
            },
        ).await
            .unwrap_or_else(|e| error_exit!("failed to launch apid: {e}"))
            .pipe(Some)
    } else {
        None
    };

    let web_child = if app_config.is_web_enabled {
        let web_socket_to_auth = web_socket_to_auth.unwrap();
        let web_socket_to_storage = web_socket_to_storage.unwrap();
        set_cloexec(&web_socket_to_auth, false);
        set_cloexec(&web_socket_to_storage, false);
        let webd_path = get_webd_executable_path()
            .unwrap_or_else(|e|
                error_exit!("failed to get webd executable path: {e}")
            );
        launch_sub(
            webd_path,
            app_config.empty_user_group.as_str_ref(),
            cli_config.is_daemonizing().into(),
            |command| {
                if let Some(ref web_rocket_config) = app_config.web_rocket_config {
                    command.arg(
                        path_arg("config-file", web_rocket_config)
                    );
                }
                command
                    .arg(
                        path_arg("public-key-file",
                        &app_config.jwt_public_key,
                    ))
                    .arg(socket_arg("auth-socket-fd", &web_socket_to_auth))
                    .arg(
                        socket_arg("storage-socket-fd", &web_socket_to_storage)
                    );
            },
        ).await
            .unwrap_or_else(|e| error_exit!("failed to launch webd: {e}"))
            .pipe(Some)
    } else {
        None
    };

    Spawns {
        daemons: vec![
            auth_child,
            storage_child,
        ],
        servers: [
            api_child,
            web_child,
        ]
            .into_iter()
            .filter_map(|v| v)
            .collect(),
    }
}

struct Spawns {
    daemons: Vec<Child>,
    servers: Vec<Child>,
}

fn path_arg(arg_name: &str, path: impl AsRef<OsStr>) -> OsString {
    let mut str = OsString::from(format!("--{arg_name}="));
    str.push(path.as_ref());
    str
}

fn socket_arg(arg_name: &str, socket: &impl AsRawFd) -> String {
    format!("--{arg_name}={}", socket.as_raw_fd().to_string())
}

async fn shutdown(
    deadline: Instant,
    children: &mut [Child],
    send_term_first: SendTerm,
) -> Vec<Result<ExitStatus, std::io::Error>> {
    join_all(
        children
            .iter_mut()
            .map(|c| c.kill_with_timeout(deadline, send_term_first))
            .collect::<Vec<_>>()
    ).await
}
