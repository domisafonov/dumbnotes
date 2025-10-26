pub mod header {
    use rocket::http::{Header, Status};
    use rocket::Responder;
    use crate::http::status::Unauthorized;

    pub struct WwwAuthenticate(Unauthorized);

    impl From<WwwAuthenticate> for Header<'static> {
        fn from(value: WwwAuthenticate) -> Self {
            Header::new(
                "WWW-Authenticate",
                format!(
                    "Bearer realm=\"users_notes\" error=\"{}\"",
                    value.0.to_error_type(),
                ),
            )
        }
    }

    #[derive(Responder)]
    #[response(status = 401)]
    pub struct UnauthorizedResponse {
        empty: (),
        www_authenticate: WwwAuthenticate,
    }

    impl From<Unauthorized> for UnauthorizedResponse {
        fn from(value: Unauthorized) -> Self {
            UnauthorizedResponse {
                empty: Default::default(),
                www_authenticate: WwwAuthenticate(value),
            }
        }
    }
}

#[allow(non_upper_case_globals)]
pub mod status {
    use rocket::http::Status;

    // TODO: https://github.com/rwf2/Rocket/issues/749
    //  use the correct way of error handling when it'd be implemented in Rocket
    #[derive(Debug, Eq, Hash, PartialEq)]
    #[repr(u16)]
    pub enum Unauthorized {
        InvalidRequest = 499,
        InvalidToken = 498,
        InsufficientScope = 497,
    }

    pub trait StatusExt {
        const UnauthorizedInvalidRequest: Status = Status::new(Unauthorized::InvalidRequest as u16);
        const UnauthorizedInvalidToken: Status = Status::new(Unauthorized::InvalidToken as u16);
        const UnauthorizedInsufficientScope: Status = Status::new(Unauthorized::InsufficientScope as u16);
    }
    impl StatusExt for Status {}

    impl Unauthorized {
        pub fn to_error_type(&self) -> &'static str {
            match self {
                Unauthorized::InvalidRequest => "invalid_request",
                Unauthorized::InvalidToken => "invalid_token",
                Unauthorized::InsufficientScope => "insufficient_scope",
            }
        }
    }
}
