use protobuf_common::{MappingError, ProtobufRequestError};
use crate::bindings;

pub struct LogoutRequest {
    pub access_token: String,
}

pub struct LogoutResponse(pub Option<bindings::LogoutError>);

impl TryFrom<bindings::LogoutRequest> for LogoutRequest {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::LogoutRequest) -> Result<Self, Self::Error> {
        Ok(
            LogoutRequest {
                access_token: value.access_token,
            }
        )
    }
}

impl TryFrom<bindings::response::Response> for LogoutResponse {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::response::Response) -> Result<Self, Self::Error> {
        let value = match value {
            bindings::response::Response::Logout(value) => value,
            _ => return Err(MappingError::UnexpectedEnumVariant.into()),
        };
        Ok(
            LogoutResponse(
                match value.error {
                    Some(e) => Some(e.try_into()?),
                    None => None,
                }
            )
        )
    }
}

impl From<LogoutResponse> for bindings::response::Response {
    fn from(value: LogoutResponse) -> Self {
        bindings::response::Response::Logout(
            bindings::LogoutResponse {
                error: value.0.map(bindings::LogoutError::into),
            }
        )
    }
}

impl From<LogoutRequest> for bindings::LogoutRequest {
    fn from(value: LogoutRequest) -> Self {
        bindings::LogoutRequest {
            access_token: value.access_token,
        }
    }
}
