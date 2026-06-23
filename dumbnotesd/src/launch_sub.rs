use std::{ops::Not, os::fd::RawFd, path::PathBuf};

use boolean_enums::gen_boolean_enum;
use dumbnotes::sandbox::user_group::get_user_and_group;
use tokio::process::{Child, Command};

pub async fn launch_sub_with_sockets(
    path: PathBuf,
    user_group: Option<&str>,
    childs_sockets: impl IntoIterator<Item=RawFd>,
    is_daemonizing: IsDaemonizing,
    command_builder: impl FnOnce(&mut Command),
) -> Result<Child, std::io::Error> {
    let mut command = Command::new(path);
    command_builder(&mut command);
    if is_daemonizing.into() && cfg!(debug_assertions) {
        command.arg("--daemonize");
    }
    if is_daemonizing.not().into() && !cfg!(debug_assertions) {
        command.arg("--no-daemonize");
    }
    if is_daemonizing.into() && let Some(user_group) = user_group
    {
        let (uid, gid) = get_user_and_group(user_group)?;
        command.uid(uid).gid(gid);
    }
    let mut childs_sockets = childs_sockets.into_iter().peekable();
    if childs_sockets.peek().is_some() {
        command
            .arg(
                format!(
                    "--socket-fds={}",
                    childs_sockets.into_iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            );
    }
    command.kill_on_drop(true);
    Ok(command.spawn()?)
}

pub async fn launch_sub(
    path: PathBuf,
    user_group: Option<&str>,
    is_daemonizing: IsDaemonizing,
    command_builder: impl FnOnce(&mut Command),
) -> Result<Child, std::io::Error> {
    launch_sub_with_sockets(
        path,
        user_group,
        [RawFd::max_value(); 0],
        is_daemonizing,
        command_builder,
    ).await
}

gen_boolean_enum!(pub IsDaemonizing);
