use std::{error::Error, str::FromStr, thread::sleep, time::{Duration, SystemTime}};

use api_data::{bindings, http::status::Unauthorized, model::{LoginRequest, LoginRequestSecret, LoginResponse}};
use data::UsernameString;
use dumbnotes::bin_constants::SESSION_ID_JWT_CLAIM_NAME;
use josekit::jwt::JwtPayload;
use reqwest::{IntoUrl, StatusCode, blocking::Response, header::WWW_AUTHENTICATE};
use tap::{Pipe, Tap};
use test_utils::{RQ, ReqwestBuilderProtoExt, ReqwestClientExt, data::MOCK_JWT_KEY_VERIFIER, setup_basic_config_with_keys_and_data};

use crate::common::{login, shutdown_assert_no_errors, spawn_daemon, url};

mod common;

#[test]
fn wrong_username() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys_and_data();
    let (mut child, reader) = spawn_daemon(&dir)?;

    for username in vec!["abcd", "dabc", "", " abc", "abc ", "abc d", "d abc"] {
        let username = UsernameString::from_str(username)?;
        let response = assert_unauth_error::<bindings::LoginRequest>(
            url("login"),
            None,
            LoginRequest {
                username,
                secret: LoginRequestSecret::Password("123".to_string()),
            },
        )?;
        assert_www_authenticate(&response, Unauthorized::InvalidToken);
    }

    shutdown_assert_no_errors(&mut child, reader)?;
    Ok(())
}

#[test]
fn wrong_password() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys_and_data();
    let (mut child, reader) = spawn_daemon(&dir)?;

    let username = UsernameString::from_str("abc")?;

    for password in vec!["1234", "4123", "", " 123", "123 ", "abc:123"] {
        let response = assert_unauth_error::<bindings::LoginRequest>(
            url("login"),
            None,
            LoginRequest {
                username: username.clone(),
                secret: LoginRequestSecret::Password(password.to_string()),
            },
        )?;
        assert_www_authenticate(&response, Unauthorized::InvalidToken);
    }

    shutdown_assert_no_errors(&mut child, reader)?;
    Ok(())
}

#[test]
fn wrong_refresh_token() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys_and_data();
    let (mut child, reader) = spawn_daemon(&dir)?;

    let username = UsernameString::from_str("abc")?;

    for token in vec![
        &b""[..],
        &[0u8; 128 / 8][..],
        "abc".as_bytes(),
    ] {
        let response = assert_unauth_error::<bindings::LoginRequest>(
            url("login"),
            None,
            LoginRequest {
                username: username.clone(),
                secret: LoginRequestSecret::RefreshToken(token.to_vec()),
            },
        )?;
        assert_www_authenticate(&response, Unauthorized::InvalidToken);
    }

    let valid_token = login(&username, "123")?.refresh_token;

    for token in vec![
        vec![],
        valid_token.clone().tap_mut(|t| t.insert(0, 0)),
        valid_token.clone().tap_mut(|t| t.push(0)),
        valid_token.clone().tap_mut(|t| t.extend(valid_token)),
        b"abc".to_vec(),
    ] {
        let response = assert_unauth_error::<bindings::LoginRequest>(
            url("login"),
            None,
            LoginRequest {
                username: username.clone(),
                secret: LoginRequestSecret::RefreshToken(token.to_vec()),
            },
        )?;
        assert_www_authenticate(&response, Unauthorized::InvalidToken);
    }

    shutdown_assert_no_errors(&mut child, reader)?;
    Ok(())
}

#[test]
fn jwt_valid() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys_and_data();
    let (mut child, reader) = spawn_daemon(&dir)?;

    let password = LoginRequestSecret::Password("123".to_string());

    let request_time = SystemTime::now() - Duration::from_secs(1);
    let (payload, _) = request_jwt(password.clone())?;

    assert_eq!(payload.subject(), Some("abc"));

    let not_before = payload.not_before().expect("missing not_before");
    let expires_at = payload.expires_at().expect("missing expires_at");
    assert!(expires_at > not_before);
    assert!(not_before > request_time);
    assert!(expires_at > request_time);

    let session_id = payload
        .claim(SESSION_ID_JWT_CLAIM_NAME)
        .expect("missing session_id");
    let new_session_id = request_jwt(password)?.0
        .claim(SESSION_ID_JWT_CLAIM_NAME)
        .expect("missing session_id")
        .to_owned();
    assert_ne!(session_id, &new_session_id);

    shutdown_assert_no_errors(&mut child, reader)?;
    Ok(())
}

#[test]
fn renewed_jwt_valid() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys_and_data();
    let (mut child, reader) = spawn_daemon(&dir)?;

    let (login_payload, login_refresh_token) = request_jwt(
        LoginRequestSecret::Password("123".to_string())
    )?;
    sleep(Duration::from_secs(1));
    let refresh_time = SystemTime::now();
    let (refresh_payload, refresh_refresh_token) = request_jwt(
        LoginRequestSecret::RefreshToken(login_refresh_token.clone())
    )?;
    assert_ne!(login_refresh_token, refresh_refresh_token);

    let login_not_before = login_payload.not_before().expect("missing not_before");
    let login_expires_at = login_payload.expires_at().expect("missing expires_at");
    let refresh_not_before = refresh_payload.not_before().expect("missing not_before");
    let refresh_expires_at = refresh_payload.expires_at().expect("missing expires_at");
    assert!(login_not_before < refresh_not_before);
    assert!(login_expires_at < refresh_expires_at);
    assert!(refresh_not_before >= refresh_time);
    assert!(refresh_expires_at >= refresh_not_before);

    shutdown_assert_no_errors(&mut child, reader)?;
    Ok(())
}

fn request_jwt(
    secret: LoginRequestSecret,
) -> Result<(JwtPayload, Vec<u8>), Box<dyn Error>> {
    let LoginResponse { access_token, refresh_token } = RQ
        .post_pb_successfully::<bindings::LoginRequest, bindings::LoginResponse>(
            url("login"),
            None,
            LoginRequest {
                username: UsernameString::from_str("abc")?,
                secret,
            }
        )?
        .try_into()?;

    Ok((
        josekit::jwt
            ::decode_with_verifier(
                access_token,
                &*MOCK_JWT_KEY_VERIFIER,
            )?
            .0,
        refresh_token,
    ))
}

#[test]
fn login_sending_auth_header() -> Result<(), Box<dyn Error>> {
    todo!()
}

#[test]
fn renew_sending_auth_header() -> Result<(), Box<dyn Error>> {
    todo!()
}

#[test]
fn multiple_logins() -> Result<(), Box<dyn Error>> {
    todo!()
}

#[test]
#[ignore = "test in a docker env with root able to set time"]
fn expired_token() -> Result<(), Box<dyn Error>> {
    todo!()
}

#[test]
#[ignore = "test in a docker env with root able to set time"]
fn expired_renew_token() -> Result<(), Box<dyn Error>> {
    todo!()
}

#[test]
#[ignore = "test in a docker env with root able to set time"]
fn renew_with_access_token_expired() -> Result<(), Box<dyn Error>> {
    todo!()
}

#[test]
fn logout() -> Result<(), Box<dyn Error>> {
    todo!()
}

#[test]
fn logout_multiple() -> Result<(), Box<dyn Error>> {
    todo!()
}

#[test]
fn request_with_invalid_auth_header() {
    todo!()
}


fn assert_unauth_error<I>(
    url: impl IntoUrl,
    auth_token: Option<&str>,
    body: impl Into<I>,
) -> Result<Response, Box<dyn Error>>
where
    I: prost::Message,
{
    RQ.post(url)
        .pipe(|builder|
            match auth_token {
                Some(token) => builder.bearer_auth(token),
                None => builder,
            }
        )
        .pb_body::<I>(body)
        .send()
        .map_err(Into::into)
        .inspect(|response|
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED)
        )
}

fn assert_www_authenticate(
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
