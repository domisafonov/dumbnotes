mod errors;
mod protobuf;
mod model;
pub mod authentication_guard;

use crate::access_granter::{AccessGranter, AccessGranterError, LoginResult};
use crate::routes::api::authentication_guard::{Authenticated, Unauthenticated};
use crate::routes::api::model::{LoginRequest, LoginRequestSecret, LoginResponse};
use rocket::http::Status;
use rocket::response::content::{RawJson, RawText};
use rocket::{get, post, routes, Route, State};

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
                Err(e) => Err(map_login_error(e))
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
                Err(e) => Err(map_login_error(e))
            }
        }
    }
}

fn map_login_error(e: AccessGranterError) -> Status {
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
}

#[post("/logout")]
async fn logout(
    authenticated: Authenticated,
    access_granter: &State<AccessGranter>,
) -> Result<(), Status> {
    match access_granter.logout_user(authenticated.0.session_id).await {
        Ok(_) => Ok(()),
        Err(_) => Err(Status::InternalServerError)
    }
}

pub fn api_routes() -> Vec<Route> {
    routes![
        version,
        login,
        logout,
    ]
}
