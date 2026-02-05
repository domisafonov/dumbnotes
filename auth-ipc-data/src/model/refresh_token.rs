use std::str::FromStr;
use data::UsernameString;
use protobuf_common::{MappingError, OptionExt, ProtobufRequestError};
use super::successful_login::SuccessfulLogin;
use crate::bindings;

pub struct RefreshTokenRequest {
    pub username: UsernameString,
    pub refresh_token: Vec<u8>,
}

pub struct RefreshTokenResponse(
    pub Result<SuccessfulLogin, bindings::LoginError>
);

impl TryFrom<bindings::RefreshTokenRequest> for RefreshTokenRequest {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::RefreshTokenRequest) -> Result<Self, Self::Error> {
        Ok(
            RefreshTokenRequest {
                username: UsernameString::from_str(&value.username)?,
                refresh_token: value.refresh_token,
            }
        )
    }
}

impl TryFrom<bindings::response::Response> for RefreshTokenResponse {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::response::Response) -> Result<Self, Self::Error> {
        use bindings::refresh_token_response::Result;
        let value = match value {
            bindings::response::Response::RefreshToken(value) => value,
            _ => return Err(MappingError::UnexpectedEnumVariant.into()),
        };
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

impl From<RefreshTokenResponse> for bindings::response::Response {
    fn from(value: RefreshTokenResponse) -> Self {
        bindings::response::Response::RefreshToken(
            bindings::RefreshTokenResponse {
                result: Some(
                    match value.0 {
                        Ok(successful_login) => 
                            bindings::refresh_token_response::Result::SuccessfulLogin(
                                successful_login.into()
                            ),
                        Err(error) => 
                            bindings::refresh_token_response::Result::LoginError(
                                error.into()
                            ),
                    }
                )
            }
        )
    }
}

impl From<RefreshTokenRequest> for bindings::RefreshTokenRequest {
    fn from(value: RefreshTokenRequest) -> Self {
        bindings::RefreshTokenRequest {
            username: value.username.into_string(),
            refresh_token: value.refresh_token,
        }
    }
}
