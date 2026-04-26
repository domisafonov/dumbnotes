use std::{error::Error, ffi::OsString, path::PathBuf, process::{Child, ChildStderr, Command, Stdio}};
use api_data::{bindings, http::status::Unauthorized, model::{LoginRequest, LoginRequestSecret, LoginResponse}};
use assert_fs::{TempDir, prelude::PathChild};
use data::UsernameStr;
use reqwest::{IntoUrl, Method, StatusCode, blocking::Response, header::WWW_AUTHENTICATE};
use tap::{Pipe, Tap};
use test_utils::{BackgroundReader, ChildKillOnDropExt, DAEMON_BIN_PATH, DAEMON_BIN_PATHS, Faketime, KillOnDropChild, LOCAL_PORT, RQ, ReqwestBuilderProtoExt, ReqwestClientExt, new_configured_command_with_env};
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

pub fn spawn_daemon_faketime(
    dir: &TempDir,
) -> Result<(KillOnDropChild, BackgroundReader<ChildStderr>, Faketime), Box<dyn Error>> {
    let (mut command, timestamp_path) = new_command_faketime(dir);
    let faketime = Faketime::new(timestamp_path)?;
    let mut child = command.spawn()?.kill_on_drop();
    let stderr = child.stderr.take()
        .expect("failed to get stderr");
    let mut reader = BackgroundReader::new(stderr, Some(30000))?;
    reader.wait_until(ROCKET_STARTED_STRING)?;
    Ok((child, reader, faketime))
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

pub fn shutdown_assert_no_errors_except(
    child: &mut Child,
    reader: BackgroundReader<ChildStderr>,
    expected_errors: &[impl AsRef<str>],
) -> Result<(), Box<dyn Error>> {
    child.kill_term()?;

    let mut expected_errors = expected_errors.into_iter();
    let mut current_error = expected_errors.next().map(AsRef::as_ref);
    let log = reader.read_to_end()?;

    for line in log.lines() {
        if let Some(expected) = current_error
            && line.matches(expected).next().is_some()
        {
            current_error = expected_errors.next().map(AsRef::as_ref);
        } else {
            assert!(
                !line.contains("ERROR"),
                "errors in the log: {log}",
            );
        }
    }

    if let Some(expected) = current_error {
        panic!("expected error {expected} not found: {log}")
    }

    assert!(child.wait()?.success());
    Ok(())
}

pub fn new_command(dir: &TempDir) -> Command {
    let mut command = new_configured_command_with_env(
        &DAEMON_BIN_PATH,
        dir,
        Some(&DAEMON_BIN_PATHS),
        [""; 0],
    );
    command.arg("--no-daemonize")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    command
}

pub fn new_command_faketime(dir: &TempDir) -> (Command, PathBuf) {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripts/run_with_faketime.fish");
    let timestamp_file = dir.child("timestamp").path().to_owned();
    let mut command = new_configured_command_with_env(
        &script,
        dir,
        Some(&DAEMON_BIN_PATHS),
        [
            OsString::from("--timestamp-file=")
                .tap_mut(|s| s.push(&timestamp_file)),
            OsString::from("--executable=")
                .tap_mut(|s| s.push(&*DAEMON_BIN_PATH)),
            OsString::from("--"),
        ],
    );
    command
        .arg("--no-daemonize")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    (command, timestamp_file)
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
    call_login(
        username,
        LoginRequestSecret::Password(password.as_ref().to_string()),
    )
}

pub fn refresh_token(
    username: impl AsRef<UsernameStr>,
    refresh_token: impl AsRef<[u8]>,
) -> Result<LoginResponse, Box<dyn Error>> {
    call_login(
        username,
        LoginRequestSecret::RefreshToken(refresh_token.as_ref().to_vec()),
    )
}

pub fn call_login(
    username: impl AsRef<UsernameStr>,
    secret: LoginRequestSecret,
) -> Result<LoginResponse, Box<dyn Error>> {
    RQ
        .post_pb_successfully::<bindings::LoginRequest, bindings::LoginResponse>(
            url("login"),
            None,
            LoginRequest {
                username: username.as_ref().to_owned(),
                secret,
            }
        )?
        .try_into()
        .map_err(Into::into)
}

pub fn assert_http_error<I>(
    method: Method,
    url: impl IntoUrl,
    auth_token: Option<&str>,
    body: impl Into<I>,
    error_code: StatusCode,
    www_authenticate: Option<Unauthorized>,
) -> Result<Response, Box<dyn Error>>
where
    I: prost::Message,
{
    assert!(error_code.is_client_error() || error_code.is_server_error());

    RQ.request(method, url)
        .pipe(|builder|
            match auth_token {
                Some(token) => builder.bearer_auth(token),
                None => builder,
            }
        )
        .pb_body::<I>(body)
        .send()
        .map_err(Into::into)
        .inspect(|response| {
            assert_eq!(response.status(), error_code);
            if let Some(www_authenticate) = www_authenticate {
                assert_www_authenticate(&response, www_authenticate);
            }
        })
}

pub fn assert_http_get_error<I>(
    url: impl IntoUrl,
    auth_token: Option<&str>,
    body: impl Into<I>,
    error_code: StatusCode,
    www_authenticate: Option<Unauthorized>,
) -> Result<Response, Box<dyn Error>>
where
    I: prost::Message,
{
    assert_http_error(
        Method::GET,
        url,
        auth_token,
        body,
        error_code,
        www_authenticate,
    )
}

pub fn assert_http_post_error<I>(
    url: impl IntoUrl,
    auth_token: Option<&str>,
    body: impl Into<I>,
    error_code: StatusCode,
    www_authenticate: Option<Unauthorized>,
) -> Result<Response, Box<dyn Error>>
where
    I: prost::Message,
{
    assert_http_error(
        Method::POST,
        url,
        auth_token,
        body,
        error_code,
        www_authenticate,
    )
}

pub fn assert_www_authenticate(
    response: &Response,
    error: Unauthorized,
) {
    let auth_header = response.headers()
        .get(WWW_AUTHENTICATE).expect("no WWW-Authenticate header")
        .to_str().expect("WWW-Authenticate header is not a valid string");
    assert_eq!(
        auth_header,
        format!(
            "Bearer realm=\"users_notes\" error=\"{}\"",
            error.to_error_type(),
        )
    );
}

pub fn assert_maybe_www_authenticate(
    response: &Response,
    error: Option<Unauthorized>,
) {
    match error {
        Some(error) => assert_www_authenticate(response, error),
        None => assert!(!response.headers().contains_key(WWW_AUTHENTICATE)),
    }
}

pub fn assert_login_error(
    username: impl AsRef<UsernameStr>,
    password: impl AsRef<str>,
    status: StatusCode,
    www_authenticate: Option<Unauthorized>,
) -> Result<(), Box<dyn Error>> {
    assert_login_error_impl(
        username,
        LoginRequestSecret::Password(password.as_ref().to_owned()),
        status,
        www_authenticate,
    )
}

pub fn assert_refresh_error(
    username: impl AsRef<UsernameStr>,
    refresh_token: impl AsRef<[u8]>,
    status: StatusCode,
    www_authenticate: Option<Unauthorized>,
) -> Result<(), Box<dyn Error>> {
    assert_login_error_impl(
        username,
        LoginRequestSecret::RefreshToken(refresh_token.as_ref().to_vec()),
        status,
        www_authenticate,
    )
}

fn assert_login_error_impl(
    username: impl AsRef<UsernameStr>,
    secret: LoginRequestSecret,
    status: StatusCode,
    www_authenticate: Option<Unauthorized>,
) -> Result<(), Box<dyn Error>> {
    assert_http_post_error::<bindings::LoginRequest>(
        url("login"),
        None,
        LoginRequest {
            username: username.as_ref().to_owned(),
            secret: secret,
        },
        status,
        www_authenticate,
    )?;
    Ok(())
}

pub fn logout(token: &str) -> Result<(), Box<dyn Error>> {
    RQ.post_pb_successfully::<(), ()>(url("logout"), Some(token), ())?;
    Ok(())
}
