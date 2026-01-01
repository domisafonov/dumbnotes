use async_trait::async_trait;
use log::error;
use rocket::{Request, State};
use rocket::http::hyper::header;
use rocket::http::Status;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome};
use crate::access_granter::{AccessGranter, AccessGranterError};
use crate::access_granter::{KnownSession, SessionInfo};
use crate::http::status::StatusExt;

#[derive(Debug)]
pub struct Unauthenticated;

#[derive(Debug)]
pub struct Authenticated(pub KnownSession);

#[derive(Debug)]
pub enum MaybeAuthenticated {
    Valid(KnownSession),
    Expired(KnownSession),
    InvalidRequest,
    InvalidToken,
    Unauthenticated,
}

#[async_trait]
impl<'r> FromRequest<'r> for Unauthenticated {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if request.headers().contains(header::AUTHORIZATION.as_str()) {
            Outcome::Forward(Status::Forbidden)
        } else {
            Outcome::Success(Unauthenticated)
        }
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for Authenticated {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match try_outcome!(request.guard::<MaybeAuthenticated>().await) {
            MaybeAuthenticated::Valid(session) => Outcome::Success(Authenticated(session)),
            MaybeAuthenticated::Expired(_) => Outcome::Error((Status::UnauthorizedInvalidToken, ())),
            MaybeAuthenticated::InvalidRequest => Outcome::Error((Status::UnauthorizedInvalidRequest, ())),
            MaybeAuthenticated::InvalidToken => Outcome::Error((Status::UnauthorizedInvalidToken, ())),
            MaybeAuthenticated::Unauthenticated => Outcome::Forward(Status::Unauthorized),
        }
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for MaybeAuthenticated {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let auth_header = match request.headers().get_one(header::AUTHORIZATION.as_str()) {
            Some(h) => h,
            _ => return Outcome::Success(MaybeAuthenticated::Unauthenticated),
        };
        let access_granter = try_outcome!(request.guard::<&State<Box<dyn AccessGranter>>>().await);
        match access_granter.check_user_access(auth_header).await {
            Ok(SessionInfo::Valid(info)) => Outcome::Success(MaybeAuthenticated::Valid(info)),
            Ok(SessionInfo::Expired(info)) => Outcome::Success(MaybeAuthenticated::Expired(info)),
            Err(e) => match e {
                AccessGranterError::HeaderFormatError
                => Outcome::Success(MaybeAuthenticated::InvalidRequest),

                AccessGranterError::InvalidToken |
                AccessGranterError::InvalidCredentials
                => Outcome::Success(MaybeAuthenticated::InvalidToken),

                AccessGranterError::ProtobufError(_) |
                AccessGranterError::Caller(_) |
                AccessGranterError::AuthDaemonInternalError
                => {
                    // TODO: forward the error when it'll be possible
                    error!("authentication system failed: {e}");
                    Outcome::Error((Status::InternalServerError, ()))
                },
            }
        }
    }
}
