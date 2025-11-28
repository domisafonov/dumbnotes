use std::str::FromStr;
use crate::username_string::UsernameString;
use crate::protobuf::{MappingError, OptionExt, ProtobufRequestError};
use super::successful_login::SuccessfulLogin;
use super::super::protobuf;

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

impl TryFrom<protobuf::RefreshTokenResponse> for RefreshTokenResponse {
    type Error = ProtobufRequestError;
    fn try_from(value: protobuf::RefreshTokenResponse) -> Result<Self, Self::Error> {
        use protobuf::refresh_token_response::Result;
        Ok(
            RefreshTokenResponse(
                match value.result.ok_or_mapping_error(MappingError::missing("result"))? {
                    Result::SuccessfulLogin(successful_login) => Ok(successful_login.into()),
                    Result::LoginError(login_error) => Err(login_error.try_into()?),
                }
            )
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

impl From<RefreshTokenRequest> for protobuf::RefreshTokenRequest {
    fn from(value: RefreshTokenRequest) -> Self {
        protobuf::RefreshTokenRequest {
            username: value.username.into_string(),
            refresh_token: value.refresh_token,
        }
    }
}
