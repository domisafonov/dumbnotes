pub mod protobuf {
    include!(concat!(env!("OUT_DIR"), "/dumbnotes.protobuf.rs"));
}

use std::error::Error;
use std::fmt::{Display, Formatter};
use async_trait::async_trait;
use prost::{DecodeError, Message};
use rocket::response::content::{RawJson, RawText};
use rocket::{get, post, routes, Data, Request, Route};
use rocket::data::{FromData, Outcome};
use rocket::http::{ContentType, Status};
use crate::routes::api::protobuf::Login;

#[get("/version")]
fn version() -> RawText<&'static str> {
    RawText("1")
}

#[post("/login", data = "<login>")]
fn login(login: Login) -> RawJson<&'static str> {
    RawJson("{}")
}

#[post("/login/refresh")]
fn login_refresh() -> RawJson<&'static str> {
    RawJson("{}")
}

#[post("/logout")]
fn logout() -> RawJson<&'static str> {
    RawJson("{}")
}

pub fn api_routes() -> Vec<Route> {
    routes![
        version,
        login,
        login_refresh,
        logout,
    ]
}

#[derive(Debug)]
pub enum ProtobufRequestError {
    DecodeError(DecodeError),
    IoError(std::io::Error),
}

impl Display for ProtobufRequestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtobufRequestError::DecodeError(e) => f.write_fmt(format_args!("{}", e)),
            ProtobufRequestError::IoError(e) => f.write_fmt(format_args!("{}", e)),
        }
    }
}

impl Error for ProtobufRequestError {}

impl From<DecodeError> for ProtobufRequestError {
    fn from(err: DecodeError) -> Self {
        ProtobufRequestError::DecodeError(err)
    }
}

impl From<std::io::Error> for ProtobufRequestError {
    fn from(err: std::io::Error) -> Self {
        ProtobufRequestError::IoError(err)
    }
}

#[async_trait]
impl<'r> FromData<'r> for Login {
    type Error = ProtobufRequestError;

    // TODO: properly
    async fn from_data(
        req: &'r Request<'_>,
        data: Data<'r>,
    ) -> rocket::data::Outcome<'r, Self> {
        if req.content_type() != Some(&ContentType::new("application", "protobuf")) {
            return Outcome::Forward((data, Status::UnsupportedMediaType))
        }

        let result = data.open(32_768.into()).into_bytes().await;
        match result {
            Err(e) => Outcome::Error((Status::BadRequest, e.into())),
            Ok(bytes) => match Login::decode(bytes.as_ref()) {
                Ok(login) => Outcome::Success(login),
                Err(e) => Outcome::Error((Status::BadRequest, e.into()))
            },
        }
    }
}
