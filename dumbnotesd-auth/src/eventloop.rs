use std::sync::Arc;

use clap::crate_name;
use protobuf_common::ProtobufRequestError;
use toml::value::Array;
use crate::session_storage::SessionStorage;
use crate::user_db::UserDb;
use tokio::net::unix::OwnedWriteHalf;
use futures::Stream;
use dumbnotes::{access_token::AccessTokenValidator, bin_constants::IPC_MESSAGE_MAX_SIZE, gen_proto_ipc_wrappers, ipc::data::{LoopInputMessage, LoopStreamExt}};
use crate::processors;
use auth_ipc_data::bindings;
use crate::access_token_generator::AccessTokenGenerator;

pub struct State<U: UserDb, S: SessionStorage> {
    pub token_generator: AccessTokenGenerator,
    pub user_db: U,
    pub session_storage: S,
    pub access_token_validator: AccessTokenValidator,
}

pub async fn process_commands<U, S>(
    // token_generator: AccessTokenGenerator,
    // user_db: impl UserDb + 'static,
    // session_storage: impl SessionStorage + 'static,
    state: Arc<State<U, S>>,
    commands: impl Stream<Item = LoopInputMessage<bindings::Command>>,
    write_socket: OwnedWriteHalf,
)
where
    U: UserDb + 'static,
    S: SessionStorage + 'static,
{
    use dumbnotes::ipc::eventloop::process_commands;

    process_commands(
        crate_name!(),
        commands.map_loop_message(Command),
        state,
        write_socket,
        dispatch_command,
        IPC_MESSAGE_MAX_SIZE,
    ).await;
}

async fn dispatch_command<U: UserDb, S: SessionStorage>(
    command: bindings::command::Command,
    state: Arc<State<U, S>>,
) -> Result<Response, ProtobufRequestError> {
    use bindings::command::Command as CE;
    let response = match command {
        CE::Login(request) => processors::process_login(
            &state.user_db,
            &state.session_storage,
            &state.token_generator,
            request.try_into()?,
        ).await,
        CE::RefreshToken(request) => processors::process_refresh_token(
            &state.session_storage,
            &state.token_generator,
            request.try_into()?,
        ).await,
        CE::Logout(request) => processors::process_logout(
            &state.session_storage,
            &state.access_token_validator,
            request.try_into()?,
        ).await,
    };
    Ok(Response(response))
}

gen_proto_ipc_wrappers!(
    bindings::Command[command] | bindings::command::Command => Command,
    bindings::Response[response] | bindings::response::Response => Response,
);
