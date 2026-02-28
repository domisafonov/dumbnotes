//! Happy path tests

use std::error::Error;
use std::str::FromStr;
use data::UsernameString;
use test_utils::RQ;
use test_utils::ReqwestClientExt;
use test_utils::setup_basic_config_with_keys_and_data;
use api_data::model::*;
use api_data::bindings;
use time::UtcDateTime;
use uuid::Uuid;
use crate::common::login;
use crate::common::refresh_token;
use crate::common::shutdown_assert_no_errors;
use crate::common::spawn_daemon;
use crate::common::url;

mod common;

#[test]
fn login_renew_logout() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys_and_data();
    let (mut child, reader) = spawn_daemon(&dir)?;

    let username = UsernameString::from_str("abc")?;

    let response = login(&username, "123")?;
    let response = refresh_token(username, response.refresh_token)?;

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
    let note_id = Uuid::new_v4();
    let mtime = UtcDateTime::from_unix_timestamp(1234567)?;

    let access_token = Some(login(username, "123")?.access_token);

    assert_eq!(get_list_length(access_token.as_deref())?, 0);

    RQ
        .put_pb_successfully::<bindings::NoteWriteRequest, ()>(
            url(&format!("notes/{note_id}")),
            access_token.as_deref(),
            NoteWriteRequest {
                name: Some("a title".to_string()),
                mtime,
                contents: "of a note".to_string(),
            },
        )?;

    assert_eq!(get_list_length(access_token.as_deref())?, 1);

    let read_note: NoteResponse = RQ
        .get_pb_successfully::<bindings::NoteResponse>(
            url(&format!("notes/{note_id}")),
            access_token.as_deref(),
        )?
        .try_into()?;
    assert_eq!(read_note.0.metadata.id, note_id);
    // TODO: check when implemented
    // assert_eq!(read_note.0.metadata.mtime, mtime);
    assert_eq!(read_note.0.name.as_deref(), Some("a title"));
    assert_eq!(read_note.0.contents, "of a note");

    RQ
        .delete_pb_successfully::<(), ()>(
            url(&format!("notes/{note_id}")),
            access_token.as_deref(),
            ()
        )?;

    assert_eq!(get_list_length(access_token.as_deref())?, 0);

    shutdown_assert_no_errors(&mut child, reader)?;
    Ok(())
}

fn get_list_length(
    token: Option<&str>,
) -> Result<usize, Box<dyn Error>> {
    let note_list: NoteListResponse = RQ
        .get_pb_successfully::<bindings::NoteListResponse>(
            url("notes"),
            token,
        )?
        .try_into()?;
    Ok(note_list.notes_info.len())
}
