use std::io;
use std::sync::Arc;
use clap::crate_name;
use crate::session_storage::SessionStorage;
use crate::user_db::UserDb;
use tokio::sync::Mutex;
use tokio::net::unix::OwnedWriteHalf;
use dumbnotes::protobuf::ProtobufRequestError;
use futures::{pin_mut, Stream};
use log::{debug, error, info, trace};
use scc::HashSet;
use tokio_stream::StreamExt;
use prost::Message;
use tokio::io::AsyncWriteExt;
use dumbnotes::error_exit;
use dumbnotes::ipc::auth::protobuf;
use crate::processors;
use dumbnotes::ipc::auth::protobuf::command::Command;
use crate::access_token_generator::AccessTokenGenerator;

pub async fn process_commands(
    token_generator: AccessTokenGenerator,
    user_db: impl UserDb + 'static,
    session_storage: impl SessionStorage + 'static,
    commands: impl Stream<Item=protobuf::Command>,
    write_socket: OwnedWriteHalf,
) {
    info!("{} listening to commands", crate_name!());

    struct State<U: UserDb, S: SessionStorage> {
        token_generator: AccessTokenGenerator,
        user_db: U,
        session_storage: S,
        write_socket: Mutex<OwnedWriteHalf>,
    }

    let state_owner = Arc::new(
        State {
            token_generator,
            user_db,
            session_storage,
            write_socket: Mutex::new(write_socket),
        }
    );

    let active_request_ids = Arc::new(HashSet::<u64>::new());

    pin_mut!(commands);
    while let Some(command) = commands.next().await {
        trace!("received command: {command:?}");

        let command_id = command.command_id;

        if active_request_ids.insert_sync(command_id).is_err() {
            error!("duplicate command id: {}", command_id);
            continue
        }
        let command = match command.command {
            Some(command) => command,
            None => {
                error!("empty command with id {}", command_id);
                active_request_ids.remove_sync(&command_id);
                continue
            },
        };

        let state = state_owner.clone();
        let active_request_ids = active_request_ids.clone();
        tokio::spawn(async move {
            let State {
                ref token_generator,
                ref user_db,
                ref session_storage,
                ref write_socket,
            } = *state;

            let result = dispatch_command(
                command_id,
                command,
                token_generator,
                user_db,
                session_storage,
                write_socket,
            ).await;

            if let Err(e) = result {
                error!("failed parsing command {command_id}: {e}");
            } else {
                debug!("command {command_id} executed successfully");
            }

            active_request_ids.remove_sync(&command_id);
        });
    }

    debug!("command connection closed");
}

async fn dispatch_command(
    command_id: u64,
    command: Command,
    token_generator: &AccessTokenGenerator,
    user_db: &impl UserDb,
    session_storage: &impl SessionStorage,
    write_socket: &Mutex<OwnedWriteHalf>,
) -> Result<(), ProtobufRequestError> {
    use dumbnotes::ipc::auth::protobuf::command::Command as CE;
    let response = match command {
        CE::Login(request) => processors::process_login(
            user_db,
            session_storage,
            token_generator,
            request.try_into()?,
        ).await,
        CE::RefreshToken(request) => processors::process_refresh_token(
            session_storage,
            token_generator,
            request.try_into()?,
        ).await,
        CE::Logout(request) => processors::process_logout(
            session_storage,
            request.try_into()?,
        ).await,
    };
    let response = protobuf::Response {
        command_id,
        response: Some(response),
    };
    write_response(
        &mut *write_socket.lock().await,
        response,
    ).await
        .unwrap_or_else(|e| error_exit!("error writing to the control socket: {e}"));
    Ok(())
}

async fn write_response(
    write_socket: &mut OwnedWriteHalf,
    response: protobuf::Response,
) -> io::Result<()> {
    let response = response.encode_to_vec();
    write_socket.write_u64(response.len() as u64).await?;
    write_socket.write_all(response.as_slice()).await?;
    Ok(())
}
