use uuid::Uuid;
use dumbnotes::protobuf::ProtobufRequestError;
use crate::protobuf;

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

impl From<LogoutResponse> for protobuf::response::Response {
    fn from(value: LogoutResponse) -> Self {
        protobuf::response::Response::Logout(
            protobuf::LogoutResponse {
                error: value.0.map(protobuf::LogoutError::into),
            }
        )
    }
}