use uuid::Uuid;
use crate::protobuf::{MappingError, ProtobufRequestError};
use super::super::protobuf;

pub struct LogoutRequest {
    pub session_id: Uuid,
}

pub struct LogoutResponse(pub Option<protobuf::LogoutError>);

impl TryFrom<protobuf::LogoutRequest> for LogoutRequest {
    type Error = ProtobufRequestError;
    fn try_from(value: protobuf::LogoutRequest) -> Result<Self, Self::Error> {
        Ok(
            LogoutRequest {
                session_id: Uuid::from_slice(&value.session_id)?,
            }
        )
    }
}

impl TryFrom<protobuf::response::Response> for LogoutResponse {
    type Error = ProtobufRequestError;
    fn try_from(value: protobuf::response::Response) -> Result<Self, Self::Error> {
        let value = match value {
            protobuf::response::Response::Logout(value) => value,
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

impl From<LogoutResponse> for protobuf::response::Response {
    fn from(value: LogoutResponse) -> Self {
        protobuf::response::Response::Logout(
            protobuf::LogoutResponse {
                error: value.0.map(protobuf::LogoutError::into),
            }
        )
    }
}

impl From<LogoutRequest> for protobuf::LogoutRequest {
    fn from(value: LogoutRequest) -> Self {
        protobuf::LogoutRequest {
            session_id: value.session_id.into_bytes().to_vec(),
        }
    }
}
