//! Happy path tests

use std::error::Error;
use std::io::Read;
use test_utils::{BackgroundReader, ChildKillOnDropExt, RQ, setup_basic_config_with_keys_and_data};
use unix::ChildKillTermExt;

mod common;

use crate::common::ROCKET_STARTED_STRING;
use crate::common::new_command;
use crate::common::shutdown_assert_no_errors;
use crate::common::spawn_daemon;
use crate::common::url;

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
    let mut response = RQ
        .get(url("version"))
        .send()?
        .error_for_status()?;
    let mut body = String::new();
    response.read_to_string(&mut body)?;
    assert_eq!(body, "1");
    shutdown_assert_no_errors(&mut child, reader)?;
    Ok(())
}
