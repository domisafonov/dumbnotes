mod errors;
mod protobuf;
mod model;
pub mod authentication_guard;

use crate::access_granter::{AccessGranter, AccessGranterError, LoginResult};
use crate::http::header::UnauthorizedResponse;
use crate::http::status::{StatusExt, Unauthorized};
use crate::routes::api::authentication_guard::{Authenticated, Unauthenticated};
use crate::routes::api::model::{LoginRequest, LoginRequestSecret, LoginResponse};
use rocket::http::Status;
use rocket::response::content::RawText;
use rocket::{catch, catchers, get, post, routes, Build, Catcher, Rocket, Route, State};
use crate::app_constants::API_PREFIX;

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
    match e {
        AccessGranterError::HeaderFormatError
        => Status::UnauthorizedInvalidRequest,

        AccessGranterError::InvalidToken |
        AccessGranterError::InvalidCredentials
        => Status::UnauthorizedInvalidToken,

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
