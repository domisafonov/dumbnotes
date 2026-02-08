use std::{error::Error, process::{Child, ChildStderr, Command, Stdio}};
use assert_fs::TempDir;
use test_utils::{BackgroundReader, ChildKillOnDropExt, DAEMON_BIN_PATH, DAEMON_BIN_PATHS, KillOnDropChild, new_configured_command_with_env};
use unix::ChildKillTermExt;

pub const ROCKET_STARTED_STRING: &str = "Rocket has launched from";

pub fn spawn_daemon(
    dir: &TempDir,
) -> Result<(KillOnDropChild, BackgroundReader<ChildStderr>), Box<dyn Error>> {
    let mut child = new_command(dir).spawn()?.kill_on_drop();
    let stderr = child.stderr.take()
        .expect("failed to get stderr");
    let mut reader = BackgroundReader::new(stderr, Some(30000))?;
    reader.wait_until(ROCKET_STARTED_STRING)?;
    Ok((child, reader))
}

pub fn shutdown_assert_no_errors(
    child: &mut Child,
    reader: BackgroundReader<ChildStderr>,
) -> Result<(), Box<dyn Error>> {
    child.kill_term()?;
    let log = reader.read_to_end()?;
    assert!(
        !log.contains("ERROR"),
        "errors in the log: {log}",
    );
    assert!(child.wait()?.success());
    Ok(())
}

pub fn new_command(dir: &TempDir) -> Command {
    let mut command = new_configured_command_with_env(
        &DAEMON_BIN_PATH,
        dir,
        Some(&DAEMON_BIN_PATHS),
    );
    command.arg("--no-daemonize")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    command
}

pub fn url(endpoint: &str) -> String {
    format!("http://localhost:8000/api/{endpoint}")
}
