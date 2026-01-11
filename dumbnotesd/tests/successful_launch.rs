//! Happy path tests

use std::error::Error;
use std::io;
use std::io::Read;
use std::process::{ChildStderr, Command, Stdio};
use assert_fs::TempDir;
use reqwest::blocking as rq;
use rexpect::spawn_stream;
use test_utils::{make_path_for_bins, new_configured_command, setup_basic_config_with_keys_and_data, BackgroundReader, ChildKillOnDropExt, KillOnDropChild, DAEMON_BIN_PATH, DAEMON_BIN_PATHS};
use unix::ChildKillTermExt;

const ROCKET_STARTED_STRING: &str = "Rocket has launched from";

#[test]
fn launch_and_stop() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys_and_data();
    let mut child = new_command(&dir).spawn()?;
    let stderr = child.stderr.take()
        .expect("failed to get stderr");
    let mut session = spawn_stream(stderr, io::empty(), Some(5000));
    let prev = session.exp_string(ROCKET_STARTED_STRING)?;
    let authd_listening_str = "dumbnotesd-auth listening to commands";
    if !prev.contains(authd_listening_str) {
        session.exp_string(authd_listening_str)?;
    }
    child.kill_term()?;
    let remaining_output = session.exp_eof()?;
    assert!(child.wait()?.success());
    assert!(!remaining_output.contains("ERROR"));
    Ok(())
}

#[test]
fn request_processed_without_errors() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys_and_data();
    let (mut child, reader) = spawn_daemon(&dir)?;
    let client = rq::Client::new();
    let mut response = String::new();
    client.get("http://localhost:8000/api/version")
        .send()?.read_to_string(&mut response)?;
    assert_eq!(response, "1");
    child.kill_term()?;
    let log = String::from_utf8(reader.read_to_end()?)?;
    assert!(child.wait()?.success());
    assert!(
        !log.contains("ERROR"),
        "errors in the log: {log}",
    );
    Ok(())
}

fn spawn_daemon(
    dir: &TempDir,
) -> Result<(KillOnDropChild, BackgroundReader<ChildStderr>), Box<dyn Error>> {
    let mut child = new_command(dir).spawn()?.kill_on_drop();
    let stderr = child.stderr.take()
        .expect("failed to get stderr");
    let mut reader = BackgroundReader::new(stderr, Some(30000))?;
    reader.wait_until(ROCKET_STARTED_STRING.as_bytes())?;
    Ok((child, reader))
}

fn new_command(dir: &TempDir) -> Command {
    let mut command = new_configured_command(&DAEMON_BIN_PATH, dir);
    command.arg("--no-daemonize")
        .env(
            "PATH",
            make_path_for_bins(&DAEMON_BIN_PATHS)
                .unwrap_or_else(|e|
                    panic!("failed to create new PATH: {e}")
                )
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    command
}
