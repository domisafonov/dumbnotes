use std::str::FromStr;
use data::UsernameString;
use crate::{protobuf_request, protobuf_response};
use crate::model::{LoginRequest, LoginRequestSecret, LoginResponse};
use protobuf_common::{MappingError, OptionExt, ProtobufRequestError};
use crate::bindings;

impl TryFrom<bindings::LoginRequest> for LoginRequest {
    type Error = ProtobufRequestError;

    fn try_from(pb: bindings::LoginRequest) -> Result<Self, Self::Error> {
        use bindings::login_request::Secret as PbSecret;

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
