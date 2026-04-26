use std::{error::Error, str::FromStr, thread::sleep, time::{Duration, SystemTime}};

use api_data::{bindings, http::status::Unauthorized, model::{LoginRequest, LoginRequestSecret, LoginResponse}};
use data::UsernameString;
use dumbnotes::bin_constants::SESSION_ID_JWT_CLAIM_NAME;
use josekit::jwt::JwtPayload;
use reqwest::StatusCode;
use tap::Tap;
use test_utils::{RQ, data::MOCK_JWT_KEY_VERIFIER, setup_basic_config_with_keys_and_data};

use crate::common::{assert_http_post_error, assert_login_error, assert_maybe_www_authenticate, assert_refresh_error, assert_www_authenticate, call_login, login, logout, refresh_token, shutdown_assert_no_errors, shutdown_assert_no_errors_except, spawn_daemon, spawn_daemon_faketime, url};

mod common;

#[test]
fn wrong_username() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys_and_data();
    let (mut child, reader) = spawn_daemon(&dir)?;

    for username in vec!["abcd", "dabc", "", " abc", "abc ", "abc d", "d abc"] {
        assert_login_error(
            UsernameString::from_str(username)?,
            "123".to_string(),
            StatusCode::UNAUTHORIZED,
            Some(Unauthorized::InvalidToken),
        )?;
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
        assert_login_error(
            &username,
            password,
            StatusCode::UNAUTHORIZED,
            Some(Unauthorized::InvalidToken),
        )?;
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
        assert_refresh_error(
            &username,
            token,
            StatusCode::UNAUTHORIZED,
            Some(Unauthorized::InvalidToken),
        )?;
    }

    let valid_token = login(&username, "123")?.refresh_token;

    for token in vec![
        vec![],
        valid_token.clone().tap_mut(|t| t.insert(0, 0)),
        valid_token.clone().tap_mut(|t| t.push(0)),
        valid_token.clone().tap_mut(|t| t.extend(valid_token)),
        b"abc".to_vec(),
    ] {
        assert_refresh_error(
            &username,
            token,
            StatusCode::UNAUTHORIZED,
            Some(Unauthorized::InvalidToken),
        )?;
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
    let LoginResponse { access_token, refresh_token } = call_login(
        UsernameString::from_str("abc")?,
        secret,
    )?;
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
    let dir = setup_basic_config_with_keys_and_data();
    let (mut child, reader) = spawn_daemon(&dir)?;

    let username = UsernameString::from_str("abc")?;

    let LoginResponse { access_token, .. } = login(
        &username,
        "123".to_string(),
    )?;
    assert_http_post_error::<bindings::LoginRequest>(
        url("login"),
        Some(&access_token),
        LoginRequest {
            username: username.clone(),
            secret: LoginRequestSecret::Password("123".to_string()),
        },
        StatusCode::FORBIDDEN,
        None,
    )?;
    assert_http_post_error::<bindings::LoginRequest>(
        url("login"),
        Some("invalid"),
        LoginRequest {
            username: username.clone(),
            secret: LoginRequestSecret::Password("123".to_string()),
        },
        StatusCode::FORBIDDEN,
        None,
    )?;

    shutdown_assert_no_errors_except(
        &mut child,
        reader,
        &["No matching routes for POST /api/login application/protobuf"; 2],
    )?;
    Ok(())
}

#[test]
fn renew_sending_auth_header() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys_and_data();
    let (mut child, reader) = spawn_daemon(&dir)?;

    let username = UsernameString::from_str("abc")?;

    let login = login(
        &username,
        "123".to_string(),
    )?;
    assert_http_post_error::<bindings::LoginRequest>(
        url("login"),
        Some(&login.access_token),
        LoginRequest {
            username: username.clone(),
            secret: LoginRequestSecret::RefreshToken(
                login.refresh_token,
            ),
        },
        StatusCode::FORBIDDEN,
        None,
    )?;
    assert_http_post_error::<bindings::LoginRequest>(
        url("login"),
        Some("invalid"),
        LoginRequest {
            username: username.clone(),
            secret: LoginRequestSecret::RefreshToken(
                "123".as_bytes().to_owned()
            ),
        },
        StatusCode::FORBIDDEN,
        None,
    )?;

    shutdown_assert_no_errors_except(
        &mut child,
        reader,
        &["No matching routes for POST /api/login application/protobuf"; 2],
    )?;
    Ok(())
}

#[test]
fn multiple_logins_tokens_are_unrelated_in_access_and_logout() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys_and_data();
    let (mut child, reader) = spawn_daemon(&dir)?;

    let abc_name = UsernameString::from_str("abc")?;
    let abcdef_name = UsernameString::from_str("abcdef")?;

    let login_abc_1 = login(
        &abc_name,
        "123".to_string(),
    )?;
    let login_abcdef_1 = login(
        &abcdef_name,
        "012".to_string(),
    )?;
    let login_abc_2 = login(
        &abc_name,
        "123".to_string(),
    )?;
    let login_abcdef_2 = login(
        &abcdef_name,
        "012".to_string(),
    )?;

    assert_ne!(login_abc_1.access_token, login_abc_2.access_token);
    assert_ne!(login_abc_1.access_token, login_abcdef_1.access_token);
    assert_ne!(login_abc_1.access_token, login_abcdef_2.access_token);
    assert_ne!(login_abc_2.access_token, login_abcdef_1.access_token);
    assert_ne!(login_abc_2.access_token, login_abcdef_2.access_token);
    assert_ne!(login_abcdef_1.access_token, login_abcdef_2.access_token);
    assert_ne!(login_abc_1.refresh_token, login_abc_2.refresh_token);
    assert_ne!(login_abc_1.refresh_token, login_abcdef_1.refresh_token);
    assert_ne!(login_abc_1.refresh_token, login_abcdef_2.refresh_token);
    assert_ne!(login_abc_2.refresh_token, login_abcdef_1.refresh_token);
    assert_ne!(login_abc_2.refresh_token, login_abcdef_2.refresh_token);
    assert_ne!(login_abcdef_1.refresh_token, login_abcdef_2.refresh_token);

    assert_refresh_error(
        &abc_name,
        &login_abcdef_1.refresh_token,
        StatusCode::UNAUTHORIZED,
        Some(Unauthorized::InvalidToken),
    )?;
    assert_refresh_error(
        &abc_name,
        &login_abcdef_2.refresh_token,
        StatusCode::UNAUTHORIZED,
        Some(Unauthorized::InvalidToken),
    )?;
    assert_refresh_error(
        &abcdef_name,
        &login_abc_1.refresh_token,
        StatusCode::UNAUTHORIZED,
        Some(Unauthorized::InvalidToken),
    )?;
    assert_refresh_error(
        &abcdef_name,
        &login_abc_2.refresh_token,
        StatusCode::UNAUTHORIZED,
        Some(Unauthorized::InvalidToken),
    )?;

    logout(&login_abc_1.access_token)?;
    assert_refresh_error(
        &abc_name,
        login_abc_1.refresh_token,
        StatusCode::UNAUTHORIZED,
        None,
    )?;
    let login_abcdef_1 = refresh_token(
        &abcdef_name,
        login_abcdef_1.refresh_token,
    )?;
    let login_abc_2 = refresh_token(
        &abc_name,
        login_abc_2.refresh_token,
    )?;
    let login_abcdef_2 = refresh_token(
        &abcdef_name,
        login_abcdef_2.refresh_token,
    )?;

    logout(&login_abc_2.access_token)?;
    assert_refresh_error(
        &abc_name,
        login_abc_2.refresh_token,
        StatusCode::UNAUTHORIZED,
        Some(Unauthorized::InvalidToken),
    )?;
    let login_abcdef_1 = refresh_token(
        &abcdef_name,
        login_abcdef_1.refresh_token,
    )?;
    let login_abcdef_2 = refresh_token(
        &abcdef_name,
        login_abcdef_2.refresh_token,
    )?;

    logout(&login_abcdef_1.access_token)?;
    assert_refresh_error(
        &abcdef_name,
        login_abcdef_1.refresh_token,
        StatusCode::UNAUTHORIZED,
        Some(Unauthorized::InvalidToken),
    )?;
    let login_abcdef_2 = refresh_token(
        &abcdef_name,
        login_abcdef_2.refresh_token,
    )?;

    logout(&login_abcdef_2.access_token)?;
    assert_refresh_error(
        &abcdef_name,
        login_abcdef_2.refresh_token,
        StatusCode::UNAUTHORIZED,
        Some(Unauthorized::InvalidToken),
    )?;

    shutdown_assert_no_errors(&mut child, reader)?;
    Ok(())
}

#[test]
#[ignore = "test faking time"]
fn expired_token() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys_and_data();
    let (mut child, reader, faketime) = spawn_daemon_faketime(&dir)?;

    // TODO

    shutdown_assert_no_errors(&mut child, reader)?;
    drop(faketime);
    Ok(())
}

#[test]
#[ignore = "test faking time"]
fn expired_renew_token() -> Result<(), Box<dyn Error>> {
    todo!()
}

#[test]
#[ignore = "test faking time"]
fn renew_with_access_token_expired() -> Result<(), Box<dyn Error>> {
    todo!()
}

#[test]
fn request_with_invalid_auth_header() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys_and_data();
    let (mut child, reader) = spawn_daemon(&dir)?;

    let login = login(UsernameString::from_str("abc")?, "123")?;

    assert_logout_failed(
        "AuthorizationHaha",
        format!("Bearer {}", &login.access_token),
        None,
    )?;
    assert_logout_failed(
        "HahaAuthorization",
        format!("Bearer {}", &login.access_token),
        None,
    )?;

    assert_logout_failed(
        "Authorization",
        format!("HahaBearer {}", &login.access_token),
        Some(Unauthorized::InvalidRequest),
    )?;
    assert_logout_failed(
        "Authorization",
        format!("BearerHaha {}", &login.access_token),
        Some(Unauthorized::InvalidRequest),
    )?;

    assert_logout_failed(
        "Authorization",
        "Bearer 123",
        Some(Unauthorized::InvalidToken),
    )?;
    assert_logout_failed(
        "Authorization",
        format!("Bearer {}$$$", &login.access_token),
        Some(Unauthorized::InvalidToken),
    )?;
    assert_logout_failed(
        "Authorization",
        format!("Bearer $$${}", &login.access_token),
        Some(Unauthorized::InvalidToken),
    )?;
    assert_logout_failed(
        "Authorization",
        format!("Bearer {}.a", &login.access_token),
        Some(Unauthorized::InvalidToken),
    )?;

    assert_logout_failed(
        "Authorization",
        format!("Bearer  {}", &login.access_token),
        Some(Unauthorized::InvalidRequest),
    )?;
    assert_logout_failed(
        "Authorization",
        format!("Bearer\t{}", &login.access_token),
        Some(Unauthorized::InvalidRequest),
    )?;
    assert_logout_failed(
        "Authorization",
        format!("Bearer\t {}", &login.access_token),
        Some(Unauthorized::InvalidRequest),
    )?;
    assert_logout_failed(
        "Authorization",
        format!("Bearer \t{}", &login.access_token),
        Some(Unauthorized::InvalidRequest),
    )?;
    assert_logout_failed(
        "Authorization",
        format!("Bearer {} a", &login.access_token),
        Some(Unauthorized::InvalidRequest),
    )?;

    // a successful run
    logout_with_header(
        "Authorization",
        format!("Bearer {}", login.access_token),
    )?
    .error_for_status()?;

    shutdown_assert_no_errors_except(
        &mut child,
        reader,
        &["No matching routes for POST /api/logout"; 2],
    )?;
    Ok(())
}

fn logout_with_header(
    header_name: impl AsRef<str>,
    header_value: impl AsRef<str>,
) -> Result<reqwest::blocking::Response, Box<dyn Error>> {
    Ok(
        RQ.post(url("logout"))
            .header(header_name.as_ref(), header_value.as_ref())
            .send()?
    )
}

fn assert_logout_failed(
    header_name: impl AsRef<str>,
    header_value: impl AsRef<str>,
    www_authenticate: Option<Unauthorized>,
) -> Result<(), Box<dyn Error>> {
    let response = logout_with_header(header_name, header_value)?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_maybe_www_authenticate(&response, www_authenticate);
    Ok(())
}
