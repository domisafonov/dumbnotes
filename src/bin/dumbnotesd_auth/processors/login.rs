use thiserror::Error;
use dumbnotes::access_token::{AccessTokenGenerator, AccessTokenGeneratorError};
use dumbnotes::session_storage::{SessionStorage, SessionStorageError};
use dumbnotes::user_db::{UserDb, UserDbError};
use log::{debug, error, info, warn};
use time::OffsetDateTime;
use dumbnotes::bin_constants::ACCESS_TOKEN_VALIDITY_TIME;
use crate::model::login::{LoginRequest, LoginResponse};
use crate::model::successful_login::SuccessfulLogin;
use crate::protobuf;
use crate::protobuf::LoginError;

pub async fn process_login(
    user_db: &impl UserDb,
    session_storage: &impl SessionStorage,
    token_generator: &AccessTokenGenerator,
    request: LoginRequest,
) -> protobuf::response::Response {
    process_login_impl(
        user_db,
        session_storage,
        token_generator,
        request,
    ).await
        .unwrap_or_else(|e| {
            error!("error processing login request: {}", e);
            LoginResponse(Err(LoginError::LoginInternalError))
        })
        .into()
}

async fn process_login_impl(
    user_db: &impl UserDb,
    session_storage: &impl SessionStorage,
    token_generator: &AccessTokenGenerator,
    request: LoginRequest,
) -> Result<LoginResponse, LoginProcessorError> {
    let LoginRequest { username, password } = request;
    debug!("logging user \"{username}\" in");
    if user_db.check_user_credentials(&username, &password).await? {
        let now = OffsetDateTime::now_utc();
        let session = session_storage
            .create_session(
                &username,
                now,
                now + ACCESS_TOKEN_VALIDITY_TIME,
            )
            .await?;
        let access_token = token_generator
            .generate_token(
                session.session_id,
                &session.username,
                &now.into(),
                &session.expires_at.into(),
            )?;
        info!(
            "logged user \"{username}\" in with session \"{}\"",
            session.session_id,
        );
        Ok(
            LoginResponse(
                Ok(
                    SuccessfulLogin {
                        access_token,
                        refresh_token: session.refresh_token,
                    }
                )
            )
        )
    } else {
        warn!("invalid credentials for user \"{}\"", username);
        Ok(
            LoginResponse(
                Err(LoginError::LoginInvalidCredentials)
            )
        )
    }
}

#[derive(Debug, Error)]
enum LoginProcessorError {
    #[error("user database error: {0}")]
    UserDb(#[from] UserDbError),

    #[error("session storage error: {0}")]
    SessionStorage(#[from] SessionStorageError),

    #[error("error generating access token: {0}")]
    AccessTokenGenerator(#[from] AccessTokenGeneratorError),
}
