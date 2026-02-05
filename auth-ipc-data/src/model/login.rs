use std::str::FromStr;
use protobuf_common::{MappingError, OptionExt, ProtobufRequestError};
use data::UsernameString;
use super::successful_login::SuccessfulLogin;
use crate::bindings;

pub struct LoginRequest {
    pub username: UsernameString,
    pub password: String,
}

pub struct LoginResponse(pub Result<SuccessfulLogin, bindings::LoginError>);

impl TryFrom<bindings::LoginRequest> for LoginRequest {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::LoginRequest) -> Result<Self, Self::Error> {
        Ok(
            LoginRequest {
                username: UsernameString::from_str(&value.username)?,
                password: value.password,
            }
        )
    }
}

impl TryFrom<bindings::response::Response> for LoginResponse {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::response::Response) -> Result<Self, Self::Error> {
        use bindings::login_response::Response;
        let value = match value {
            bindings::response::Response::Login(value) => value,
            _ => return Err(MappingError::UnexpectedEnumVariant.into()),
        };
        Ok(
            LoginResponse(
                match value.response.ok_or_mapping_error(MappingError::missing("response"))? {
                    Response::SuccessfulLogin(successful_login) => Ok(successful_login.into()),
                    Response::LoginError(login_error) => Err(login_error.try_into()?),
                }
            )
        )
    }
}

impl From<LoginResponse> for bindings::response::Response {
    fn from(value: LoginResponse) -> Self {
        bindings::response::Response::Login(
            bindings::LoginResponse {
                response: Some(
                    match value.0 {
                        Ok(successful_login) =>
                            bindings::login_response::Response::SuccessfulLogin(
                                successful_login.into()
                            ),
                        Err(error) =>
                            bindings::login_response::Response::LoginError(
                                error.into()
                            ),
                    }
                ),
            }
        )
    }
}

impl From<LoginRequest> for bindings::LoginRequest {
    fn from(value: LoginRequest) -> Self {
        bindings::LoginRequest {
            username: value.username.into_string(),
            password: value.password,
        }
    }
}
