mod errors;
mod protobuf;
mod model;
pub mod authentication_guard;

use rocket::response::content::{RawJson, RawText};
use rocket::{get, post, routes, Route, State};
use rocket::http::Status;
use crate::access_granter::{AccessGranter, AccessGranterError, LoginResult};
use crate::access_token::AccessTokenGeneratorError;
use crate::routes::api::authentication_guard::{Authenticated, MaybeAuthenticated, Unauthenticated};
use crate::routes::api::model::{LoginRequest, LoginRequestSecret, LoginResponse};
use crate::session_storage::SessionStorageError;
use crate::user_db::UserDbError;

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
                Err(e) => Err(
                    match e { // TODO: headers
                        AccessGranterError::HeaderFormatError |
                        AccessGranterError::InvalidToken |
                        AccessGranterError::InvalidCredentials
                        => Status::Unauthorized,

                        AccessGranterError::SessionStorageError(_) |
                        AccessGranterError::UserDbError(_) |
                        AccessGranterError::AccessTokenGeneratorError(_)
                        => Status::InternalServerError,
                    }
                )
            }
        }
        LoginRequestSecret::RefreshToken(token) => {
            access_granter.refresh_user_token(&token).await;
            todo!()
        }
    }
}

#[post("/logout")]
fn logout(
    authenticated: Authenticated,
) -> RawJson<&'static str> {
    RawJson("{}")
}

pub fn api_routes() -> Vec<Route> {
    routes![
        version,
        login,
        logout,
    ]
}
