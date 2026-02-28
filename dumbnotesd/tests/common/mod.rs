use std::{error::Error, process::{Child, ChildStderr, Command, Stdio}};
use api_data::{bindings, model::{LoginRequest, LoginRequestSecret, LoginResponse}};
use assert_fs::TempDir;
use data::UsernameStr;
use test_utils::{BackgroundReader, ChildKillOnDropExt, DAEMON_BIN_PATH, DAEMON_BIN_PATHS, KillOnDropChild, LOCAL_PORT, RQ, ReqwestClientExt, new_configured_command_with_env};
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
    format!(
        "http://localhost:{}/api/{endpoint}",
        LOCAL_PORT.with(Clone::clone),
    )
}

pub fn login(
    username: impl AsRef<UsernameStr>,
    password: impl AsRef<str>,
) -> Result<LoginResponse, Box<dyn Error>> {
    RQ
        .post_pb_successfully::<bindings::LoginRequest, bindings::LoginResponse>(
            url("login"),
            None,
            LoginRequest {
                username: username.as_ref().to_owned(),
                secret: LoginRequestSecret::Password(password.as_ref().to_string()),
            }
        )?
        .try_into()
        .map_err(Into::into)
}

pub fn refresh_token(
    username: impl AsRef<UsernameStr>,
    refresh_token: impl AsRef<[u8]>,
) -> Result<LoginResponse, Box<dyn Error>> {
    RQ
        .post_pb_successfully::<bindings::LoginRequest, bindings::LoginResponse>(
            url("login"),
            None,
            LoginRequest {
                username: username.as_ref().to_owned(),
                secret: LoginRequestSecret::RefreshToken(refresh_token.as_ref().to_owned()),
            }
        )?
        .try_into()
        .map_err(Into::into)
}
