use std::str::FromStr;
use data::UsernameString;
use crate::{protobuf_request, protobuf_response};
use crate::model::{LoginRequest, LoginRequestSecret, LoginResponse};
use protobuf_common::{MappingError, OptionExt, ProtobufRequestError};
use crate::bindings;
use bindings::login_request::Secret as PbSecret;

impl TryFrom<bindings::LoginRequest> for LoginRequest {
    type Error = ProtobufRequestError;

    fn try_from(pb: bindings::LoginRequest) -> Result<Self, Self::Error> {
        Ok(
            LoginRequest {
                username: UsernameString::from_str(&pb.username)?,
                secret: match pb.secret.ok_or_mapping_error(
                    MappingError::missing("secret")
                )? {
                    PbSecret::Password(s) => LoginRequestSecret::Password(s),
                    PbSecret::RefreshToken(s) =>
                        LoginRequestSecret::RefreshToken(s),
                }
            }
        )
    }
}

impl From<LoginRequest> for bindings::LoginRequest {
    fn from(value: LoginRequest) -> Self {
        bindings::LoginRequest {
            username: value.username.into_string(),
            secret: Some(
                match value.secret {
                    LoginRequestSecret::Password(p) => PbSecret::Password(p),
                    LoginRequestSecret::RefreshToken(rt) => PbSecret::RefreshToken(rt),
                }
            )
        }
    }
}

impl TryFrom<bindings::LoginResponse> for LoginResponse {
    type Error = ProtobufRequestError;

    fn try_from(pb: bindings::LoginResponse) -> Result<Self, Self::Error> {
        Ok(
            LoginResponse {
                access_token: pb.token,
                refresh_token: pb.refresh_token,
            }
        )
    }
}

impl From<LoginResponse> for bindings::LoginResponse {
    fn from(value: LoginResponse) -> Self {
        bindings::LoginResponse {
            refresh_token: value.refresh_token,
            token: value.access_token,
        }
    }
}

protobuf_request!(bindings::LoginRequest, LoginRequest);
protobuf_response!(bindings::LoginResponse, LoginResponse);
