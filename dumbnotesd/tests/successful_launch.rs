use std::error::Error;
use std::process::{Command, Stdio};
use assert_fs::TempDir;
use rexpect::spawn_stream;
use test_utils::{make_path_for_bins, new_configured_command, setup_basic_config_with_keys_and_data, DAEMON_BIN_PATH, DAEMON_BIN_PATHS};
use unix::ChildExt;

#[test]
fn launch_and_stop() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys_and_data();
    let mut child = new_command(&dir).spawn()?;
    let stdin = child.stdin.take()
        .expect("failed to get stdin");
    let stderr = child.stderr.take()
        .expect("failed to get stderr");
    let mut session = spawn_stream(stderr, stdin, Some(5000));
    let prev = session.exp_string("Rocket has launched from")?;
    let authd_listening_str = "dumbnotesd-auth listening to commands";
    if !prev.contains(authd_listening_str) {
        session.exp_string(authd_listening_str)?;
    }
    child.kill_term()?;
    let remaining_output = session.exp_eof()?;
    let exit_code = child.wait()?.code()
        .expect("failed to get exit code");
    assert_eq!(exit_code, 0);
    assert!(!remaining_output.contains("ERROR"));
    Ok(())
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
        .stdin(Stdio::piped())
        .stderr(Stdio::piped());
    command
}
