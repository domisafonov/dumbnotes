use std::error::Error;
use std::str::FromStr;
use data::UsernameString;
use test_utils::RQ;
use test_utils::ReqwestClientExt;
use test_utils::setup_basic_config_with_keys_and_data;
use api_data::model::*;
use api_data::bindings;
use crate::common::shutdown_assert_no_errors;
use crate::common::spawn_daemon;
use crate::common::url;

mod common;

#[test]
fn login_renew_logout() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys_and_data();
    let (mut child, reader) = spawn_daemon(&dir)?;

    let username = UsernameString::from_str("abc")?;

    let response: LoginResponse = RQ
        .post_pb_successfully::<bindings::LoginRequest, bindings::LoginResponse>(
            url("login"),
            None,
            LoginRequest {
                username: username.clone(),
                secret: LoginRequestSecret::Password("123".to_string()),
            }
        )?
        .try_into()?;
    let response: LoginResponse = RQ
        .post_pb_successfully::<bindings::LoginRequest, bindings::LoginResponse>(
            url("login"),
            None,
            LoginRequest {
                username,
                secret: LoginRequestSecret::RefreshToken(response.refresh_token),
            },
        )?
        .try_into()?;

    RQ.post(url("logout"))
        .bearer_auth(response.access_token)
        .send()?
        .error_for_status()?;

    shutdown_assert_no_errors(&mut child, reader)?;

    Ok(())
}

#[test]
fn create_read_delete_check_deletion() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys_and_data();
    let (mut child, reader) = spawn_daemon(&dir)?;

    let username = UsernameString::from_str("abc")?;

    let LoginResponse { access_token, .. } = RQ
        .post_pb_successfully::<bindings::LoginRequest, bindings::LoginResponse>(
            url("login"),
            None,
            LoginRequest {
                username: username.clone(),
                secret: LoginRequestSecret::Password("123".to_string()),
            }
        )?
        .try_into()?;
    let access_token = Some(access_token);

    let note_list: NoteListResponse = RQ
        .get_pb_successfully::<bindings::NoteListResponse>(url("notes"), access_token)?
        .try_into()?;
    assert!(note_list.notes_info.is_empty());

    // TODO

    shutdown_assert_no_errors(&mut child, reader)?;

    Ok(())
}
