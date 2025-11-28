use std::str::FromStr;
use crate::protobuf::{MappingError, OptionExt, ProtobufRequestError};
use crate::username_string::UsernameString;
use super::successful_login::SuccessfulLogin;
use super::super::protobuf;

pub struct LoginRequest {
    pub username: UsernameString,
    pub password: String,
}

pub struct LoginResponse(pub Result<SuccessfulLogin, protobuf::LoginError>);

impl TryFrom<protobuf::LoginRequest> for LoginRequest {
    type Error = ProtobufRequestError;
    fn try_from(value: protobuf::LoginRequest) -> Result<Self, Self::Error> {
        Ok(
            LoginRequest {
                username: UsernameString::from_str(&value.username)?,
                password: value.password,
            }
        )
    }
}

impl TryFrom<protobuf::LoginResponse> for LoginResponse {
    type Error = ProtobufRequestError;
    fn try_from(value: protobuf::LoginResponse) -> Result<Self, Self::Error> {
        use protobuf::login_response::Response;
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

impl From<LoginResponse> for protobuf::response::Response {
    fn from(value: LoginResponse) -> Self {
        protobuf::response::Response::Login(
            protobuf::LoginResponse {
                response: Some(
                    match value.0 {
                        Ok(successful_login) =>
                            protobuf::login_response::Response::SuccessfulLogin(
                                successful_login.into()
                            ),
                        Err(error) =>
                            protobuf::login_response::Response::LoginError(
                                error.into()
                            ),
                    }
                ),
            }
        )
    }
}

impl From<LoginRequest> for protobuf::LoginRequest {
    fn from(value: LoginRequest) -> Self {
        protobuf::LoginRequest {
            username: value.username.into_string(),
            password: value.password,
        }
    }
}
