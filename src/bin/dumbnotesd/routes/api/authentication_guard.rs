use async_trait::async_trait;
use rocket::{Request, State};
use rocket::http::hyper::header;
use rocket::http::Status;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome};
use crate::access_granter::{AccessGranter, AccessGranterError, SessionInfo, KnownSession};

#[derive(Debug)]
pub struct Unauthenticated;

#[derive(Debug)]
pub struct Authenticated(KnownSession);

#[derive(Debug)]
pub enum MaybeAuthenticated {
    Valid(KnownSession),
    Expired(KnownSession),
    Invalid,
    Unauthenticated,
}

#[async_trait]
impl<'r> FromRequest<'r> for Unauthenticated {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if request.headers().contains(header::AUTHORIZATION.as_str()) {
            Outcome::Error((Status::Forbidden, ()))
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
            MaybeAuthenticated::Expired(_) => Outcome::Error((Status::Unauthorized, ())),
            MaybeAuthenticated::Invalid => Outcome::Error((Status::Unauthorized, ())),
            MaybeAuthenticated::Unauthenticated => Outcome::Error((Status::Unauthorized, ())),
        }
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for MaybeAuthenticated {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let auth_header = if let Some(h) = request.headers().get_one(header::AUTHORIZATION.as_str()) {
            h
        } else {
            return Outcome::Success(MaybeAuthenticated::Unauthenticated);
        };
        let access_granter = try_outcome!(request.guard::<&State<AccessGranter>>().await);
        match access_granter.check_user_access(auth_header).await {
            Ok(SessionInfo::Valid(info)) => Outcome::Success(MaybeAuthenticated::Valid(info)),
            Ok(SessionInfo::Expired(info)) => Outcome::Success(MaybeAuthenticated::Expired(info)),
            Err(e) => match e {
                AccessGranterError::HeaderFormatError |
                AccessGranterError::InvalidToken |
                AccessGranterError::InvalidCredentials
                => Outcome::Success(MaybeAuthenticated::Invalid),

                AccessGranterError::SessionStorageError(_) |
                AccessGranterError::UserDbError(_) |
                AccessGranterError::AccessTokenGeneratorError(_)
                => Outcome::Error((Status::InternalServerError, ())), // TODO: forward
            }
        }
    }
}
