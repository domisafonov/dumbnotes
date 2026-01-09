use std::error::Error;
use std::process::Command;
use assert_fs::TempDir;
use rexpect::session::spawn_command;
use test_utils::{make_path_for_bins, new_configured_command, setup_basic_config_with_keys_and_data, PtySessionExt, DAEMON_BIN_PATH, DAEMON_BIN_PATHS};

#[test]
fn launch_and_stop() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys_and_data();
    let mut child = spawn_command(new_command(&dir), Some(5000))?;
    let prev = child.exp_string("Rocket has launched from")?;
    let authd_listening_str = "dumbnotesd-auth listening to commands";
    if !prev.contains(authd_listening_str) {
        child.exp_string(authd_listening_str)?;
    }
    child.send_control('c')?;
    let remaining_output = child.assert_exit_success()?;
    assert!(!remaining_output.contains("ERROR"));
    Ok(())
}

fn new_command(dir: &TempDir) -> Command {
    let mut command = new_configured_command(&DAEMON_BIN_PATH, dir);
    command.arg("--no-daemonize");
    command.env(
        "PATH",
        make_path_for_bins(&DAEMON_BIN_PATHS)
            .unwrap_or_else(|e|
                panic!("failed to create new PATH: {e}")
            )
    );
    command
}
