use thiserror::Error;
use dumbnotes::ipc::auth::caller::CallerError;
use dumbnotes::protobuf::ProtobufRequestError;

#[derive(Debug, Error)]
pub enum AccessGranterError {
    #[error("token format error")]
    HeaderFormatError,

    #[error("invalid token")]
    InvalidToken,

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("call the auth daemon failed")]
    Caller(#[from] CallerError),

    #[error("auth daemon internal error")]
    AuthDaemonInternalError,

    #[error(transparent)]
    ProtobufError(#[from] ProtobufRequestError),
}
