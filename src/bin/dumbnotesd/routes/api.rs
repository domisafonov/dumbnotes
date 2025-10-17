mod errors;
mod protobuf;

use rocket::response::content::{RawJson, RawText};
use rocket::{get, post, routes, Route};
use crate::routes::api::protobuf::bindings::{LoginRequest, LoginResponse};

#[get("/version")]
fn version() -> RawText<&'static str> {
    RawText("1")
}

#[post("/login", data = "<login>")]
fn login(login: LoginRequest) -> LoginResponse {
    LoginResponse {
        refresh_token: Some("aaa".to_string()),
        token: "bbb".to_string(),
    }
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
