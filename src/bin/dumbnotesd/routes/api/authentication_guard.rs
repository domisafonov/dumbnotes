use async_trait::async_trait;
use rocket::{Request, State};
use rocket::http::hyper::header;
use rocket::http::Status;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome};
use crate::access_granter::{AccessGranter, AccessGranterError, SessionInfo, ValidSession};

#[derive(Debug)]
pub struct Authenticated(ValidSession);

#[async_trait]
impl<'r> FromRequest<'r> for Authenticated {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let access_granter = try_outcome!(request.guard::<&State<AccessGranter>>().await);
        let auth_header = if let Some(h) = request.headers().get_one(header::AUTHORIZATION.as_str()) {
            h
        } else {
            return Outcome::Error((Status::Unauthorized, ()));
        };
        match access_granter.check_user_access(auth_header).await {
            Ok(SessionInfo::Valid(info)) => Outcome::Success(Authenticated(info)),
            Ok(SessionInfo::Expired) => Outcome::Error((Status::Unauthorized, ())), // TODO: error messages, WWW-Authenticate
            Err(e) => Outcome::Error(
                match e {
                    AccessGranterError::HeaderFormatError => ((Status::Unauthorized, ())), // TODO: error messages, WWW-Authenticate
                    AccessGranterError::InvalidToken => ((Status::Unauthorized, ())), // TODO: error messages, WWW-Authenticate
                }
            )
        }
    }
}
