mod protobuf;
mod model;
pub mod authentication_guard;

use crate::access_granter::AccessGranter;
use crate::access_granter::AccessGranterError;
use crate::access_granter::LoginResult;
use crate::app_constants::API_PREFIX;
use crate::http::header::UnauthorizedResponse;
use crate::http::status::{StatusExt, Unauthorized};
use crate::routes::api::authentication_guard::{Authenticated, Unauthenticated};
use crate::routes::api::model::{LoginRequest, LoginRequestSecret, LoginResponse, NoteListResponse, NoteResponse, NoteWriteRequest};
use dumbnotes::storage::{NoteStorage, StorageError};
use futures::TryFutureExt;
use log::{debug, error, warn};
use rocket::http::Status;
use rocket::response::content::RawText;
use rocket::{catch, catchers, delete, get, post, put, routes, Build, Rocket, State};
use uuid::Uuid;
use dumbnotes::data::{Note, NoteMetadata};
use dumbnotes::util::send_fut_lifetime_workaround;

#[get("/version")]
fn version() -> RawText<&'static str> {
    RawText("1")
}

#[post("/login", data = "<request>")]
async fn login(
    request: LoginRequest,
    _unauthenticated: Unauthenticated,
    access_granter: &State<AccessGranter>,
) -> Result<LoginResponse, Status> {
    match request.secret {
        LoginRequestSecret::Password(password) => {
            match access_granter
                .login_user(&request.username, &password)
                .await
            {
                Ok(LoginResult { refresh_token, access_token }) => Ok(
                    LoginResponse {
                        refresh_token,
                        access_token,
                    }
                ),
                Err(e) => Err(process_login_error(e))
            }
        }
        LoginRequestSecret::RefreshToken(token) => {
            match access_granter
                .refresh_user_token(&request.username, &token)
                .await
            {
                Ok(LoginResult { refresh_token, access_token }) => Ok(
                    LoginResponse {
                        refresh_token,
                        access_token,
                    }
                ),
                Err(e) => Err(process_login_error(e))
            }
        }
    }
}

fn process_login_error(e: AccessGranterError) -> Status {
    match e {
        AccessGranterError::HeaderFormatError
        => Status::UnauthorizedInvalidRequest,

        AccessGranterError::InvalidToken |
        AccessGranterError::InvalidCredentials
        => Status::UnauthorizedInvalidToken,

        AccessGranterError::SessionStorageError(_) |
        AccessGranterError::UserDbError(_) |
        AccessGranterError::AccessTokenGeneratorError(_)
        => {
            error!("authentication system failed: {e}");
            Status::InternalServerError
        },
    }
}

#[post("/logout")]
async fn logout(
    authenticated: Authenticated,
    access_granter: &State<AccessGranter>,
) -> Result<(), Status> {
    match access_granter.logout_user(authenticated.0.session_id).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("authentication system failed: {e}");
            Err(Status::InternalServerError)
        }
    }
}

#[get("/notes")]
async fn get_users_notes(
    authenticated: Authenticated,
    note_storage: &State<NoteStorage>,
) -> Result<NoteListResponse, Status> {
    let result = note_storage
        .list_notes(&authenticated.0.username)
        .and_then(|notes_metadata| {
            note_storage
                .get_note_details(&authenticated.0.username, notes_metadata)
        })
        .await;
    match result {
        Ok(notes_info) => Ok(
            NoteListResponse {
                notes_info: notes_info
                    .into_iter()
                    .filter_map(|maybe_info| {
                        if maybe_info.is_none() {
                            warn!("no info could be read for a note");
                        }
                        maybe_info
                    })
                    .collect(),
            }
        ),
        Err(e) => {
            error!("error fetching note info: {}", e);
            Err(Status::InternalServerError)
        },
    }
}

#[get("/notes/<note_id>")]
async fn get_note(
    authenticated: Authenticated,
    note_storage: &State<NoteStorage>,
    note_id: Uuid,
) -> Result<NoteResponse, Status> {
    let result =
        send_fut_lifetime_workaround(
            note_storage.read_note(&authenticated.0.username, note_id)
        )
        .await;
    match result {
        Ok(note) => Ok(NoteResponse(note)),
        Err(e) => match e {
            StorageError::NoteNotFound => {
                debug!(
                    "no note found with id {note_id} for user \"{}\"",
                    authenticated.0.username,
                );
                Err(Status::NotFound)
            }
            _ => {
                error!("error fetching note: {}", e);
                Err(Status::InternalServerError)
            },
        }
    }
}

#[put("/notes/<note_id>", data = "<note>")]
async fn write_note(
    authenticated: Authenticated,
    note_id: Uuid,
    note: NoteWriteRequest,
    note_storage: &State<NoteStorage>,
) -> Result<(), Status> {
    let result = note_storage
        .write_note(
            &authenticated.0.username,
            &Note {
                metadata: NoteMetadata {
                    id: note_id,
                    mtime: note.mtime, // TODO: validate when we start writing it
                },
                name: note.name,
                contents: note.contents,
            }
        )
        .await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("error writing note: {}", e);
            Err(Status::InternalServerError)
        }
    }
}

#[delete("/notes/<note_id>")]
async fn delete_note(
    authenticated: Authenticated,
    note_id: Uuid,
    note_storage: &State<NoteStorage>,
) -> Result<(), Status> {
    let result = note_storage
        .delete_note(&authenticated.0.username, note_id)
        .await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("error deleting note: {}", e);
            Err(Status::InternalServerError)
        }
    }
}

#[catch(499)]
fn catch_unauthorized_invalid_request() -> UnauthorizedResponse {
    assert_eq!(Status::UnauthorizedInvalidRequest.code, 499);
    Unauthorized::InvalidRequest.into()
}

#[catch(498)]
fn catch_unauthorized_invalid_token() -> UnauthorizedResponse {
    assert_eq!(Status::UnauthorizedInvalidToken.code, 498);
    Unauthorized::InvalidToken.into()
}

#[catch(497)]
fn catch_unauthorized_insufficient_scope() -> UnauthorizedResponse {
    assert_eq!(Status::UnauthorizedInsufficientScope.code, 497);
    Unauthorized::InsufficientScope.into()
}

pub trait ApiRocketBuildExt {
    fn install_dumbnotes_api(self) -> Self;
}

impl ApiRocketBuildExt for Rocket<Build> {
    fn install_dumbnotes_api(self) -> Self {
        self
            .mount(
                API_PREFIX,
                routes![
                    version,
                    login,
                    logout,
                    get_users_notes,
                    get_note,
                    write_note,
                    delete_note,
                ],
            )
            .register(
                API_PREFIX,
                catchers![
                    catch_unauthorized_invalid_request,
                    catch_unauthorized_invalid_token,
                    catch_unauthorized_insufficient_scope,
                ]
            )
    }
}
