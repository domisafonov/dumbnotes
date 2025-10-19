mod errors;
mod protobuf;
mod model;

use rocket::response::content::{RawJson, RawText};
use rocket::{get, post, routes, Route};
use rocket::http::Status;
use crate::routes::api::model::{LoginRequest, LoginResponse};

#[get("/version")]
fn version() -> RawText<&'static str> {
    RawText("1")
}

#[post("/login", data = "<request>")]
fn login(request: LoginRequest) -> Result<LoginResponse, Status> {
    Ok(
        LoginResponse {
            refresh_token: "aaa".into(),
            token: "bbb".to_string(),
        }
    )
}

#[post("/logout")]
fn logout() -> RawJson<&'static str> {
    RawJson("{}")
}

pub fn api_routes() -> Vec<Route> {
    routes![
        version,
        login,
        logout,
    ]
}
