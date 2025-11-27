use std::str::FromStr;
use dumbnotes::username_string::UsernameString;
use dumbnotes::protobuf::ProtobufRequestError;
use crate::model::successful_login::SuccessfulLogin;
use crate::protobuf;

pub struct RefreshTokenRequest {
    pub username: UsernameString,
    pub refresh_token: Vec<u8>,
}

pub struct RefreshTokenResponse(
    pub Result<SuccessfulLogin, protobuf::LoginError>
);

impl TryFrom<protobuf::RefreshTokenRequest> for RefreshTokenRequest {
    type Error = ProtobufRequestError;
    fn try_from(value: protobuf::RefreshTokenRequest) -> Result<Self, Self::Error> {
        Ok(
            RefreshTokenRequest {
                username: UsernameString::from_str(&value.username)?,
                refresh_token: value.refresh_token,
            }
        )
    }
}

impl From<RefreshTokenResponse> for protobuf::response::Response {
    fn from(value: RefreshTokenResponse) -> Self {
        protobuf::response::Response::RefreshToken(
            protobuf::RefreshTokenResponse {
                result: Some(
                    match value.0 {
                        Ok(successful_login) => 
                            protobuf::refresh_token_response::Result::SuccessfulLogin(
                                successful_login.into()
                            ),
                        Err(error) => 
                            protobuf::refresh_token_response::Result::LoginError(
                                error.into()
                            ),
                    }
                )
            }
        )
    }
}
