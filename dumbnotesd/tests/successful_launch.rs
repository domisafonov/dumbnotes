//! Happy path tests

use std::error::Error;
use std::io::Read;
use std::process::{Child, ChildStderr, Command, Stdio};
use assert_fs::TempDir;
use test_utils::{new_configured_command_with_env, setup_basic_config_with_keys_and_data, BackgroundReader, ChildKillOnDropExt, KillOnDropChild, DAEMON_BIN_PATH, DAEMON_BIN_PATHS, RQ};
use unix::ChildKillTermExt;

const ROCKET_STARTED_STRING: &str = "Rocket has launched from";

#[test]
fn launch_and_stop() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys_and_data();
    let mut child = new_command(&dir).spawn()?.kill_on_drop();
    let stderr = child.stderr.take()
        .expect("failed to get stderr");
    let mut reader = BackgroundReader::new(stderr, Some(30000))?;
    let init1 = reader.wait_until(ROCKET_STARTED_STRING)?;
    let authd_listening_str = "dumbnotesd-auth listening to commands";
    let init2 = if !init1.contains(authd_listening_str) {
        reader.wait_until(authd_listening_str)?
    } else {
        String::new()
    };
    child.kill_term()?;
    let remaining_output = reader.read_to_end()?;
    assert!(child.wait()?.success());
    assert!(!init1.contains("ERROR"));
    assert!(!init2.contains("ERROR"));
    assert!(!remaining_output.contains("ERROR"));
    Ok(())
}

#[test]
fn request_processed_without_errors() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys_and_data();
    let (mut child, reader) = spawn_daemon(&dir)?;
    let mut response = RQ.get("http://localhost:8000/api/version").send()?;
    let mut body = String::new();
    response.read_to_string(&mut body)?;
    assert!(response.status().is_success(), "{body}");
    assert_eq!(body, "1");
    shutdown_assert_no_errors(&mut child, reader)?;
    Ok(())
}

// #[test]
// fn login_renew_logout() -> Result<(), Box<dyn Error>> {
//     let dir = setup_basic_config_with_keys_and_data();
//     let (mut child, reader) = spawn_daemon(&dir)?;
// //    crate::protobuf::LoginRequest {
// //        ..Default::default()
// //    };
//     // let request = crate::protobuf::
//     todo!();
// }

fn spawn_daemon(
    dir: &TempDir,
) -> Result<(KillOnDropChild, BackgroundReader<ChildStderr>), Box<dyn Error>> {
    let mut child = new_command(dir).spawn()?.kill_on_drop();
    let stderr = child.stderr.take()
        .expect("failed to get stderr");
    let mut reader = BackgroundReader::new(stderr, Some(30000))?;
    reader.wait_until(ROCKET_STARTED_STRING)?;
    Ok((child, reader))
}

fn shutdown_assert_no_errors(
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

fn new_command(dir: &TempDir) -> Command {
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
