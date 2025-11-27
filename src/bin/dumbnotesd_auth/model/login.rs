use std::str::FromStr;
use dumbnotes::protobuf::ProtobufRequestError;
use dumbnotes::username_string::UsernameString;
use crate::model::successful_login::SuccessfulLogin;
use crate::protobuf;

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
