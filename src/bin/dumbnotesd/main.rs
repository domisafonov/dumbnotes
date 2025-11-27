mod cli;
pub mod app_constants;
mod routes;
pub mod access_granter;
pub mod http;

use crate::access_granter::AccessGranter;
use crate::cli::CliConfig;
use dumbnotes::file_watcher::ProductionFileWatcher;
use crate::routes::{ApiRocketBuildExt, WebRocketBuildExt};
use dumbnotes::session_storage::ProductionSessionStorage;
use dumbnotes::user_db::{ProductionUserDb, UserDb};
use boolean_enums::gen_boolean_enum;
use clap::{crate_name, Parser};
use dumbnotes::access_token::{AccessTokenDecoder, AccessTokenGenerator};
use dumbnotes::config::app_config::AppConfig;
use dumbnotes::config::figment::FigmentExt;
use dumbnotes::hasher::{ProductionHasher, ProductionHasherConfig};
use dumbnotes::logging::init_logging;
use dumbnotes::storage::NoteStorage;
use figment::Figment;
use josekit::jwk::Jwk;
use log::{error, info};
use rocket::{launch, Build, Rocket};
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::Path;
use std::process::exit;
use socket2::{Domain, Socket, Type};
use tokio::net::UnixStream;
use dumbnotes::error_exit;

#[launch]
async fn rocket() -> Rocket<Build> {
    init_logging();

    info!("{} starting up", crate_name!());

    let cli_config = CliConfig::parse();

    if !cli_config.config_file.exists() {
        error_exit!(
            "configuration file at {} does not exist",
            cli_config.config_file.display()
        )
    }

    let figment = Figment::from(rocket::Config::default())
        .setup_app_config(cli_config.config_file);
    let config: AppConfig = figment.extract()
        .unwrap_or_else(|e| {
            for e in e {
                error!("{e}");
            }
            info!("finishing due to a config parse error");
            exit(1)
        });

    let hasher_config = config.hasher_config.clone().try_into().unwrap_or_else(|e| {
        error_exit!("hasher config read failed: {e}")
    });
    let hasher = ProductionHasher::new(
        ProductionHasherConfig::new(hasher_config),
    );

    // TODO: WIP, rewrite properly
    let (socket_to_auth, auth_childs_socket) = Socket
        ::pair_raw(Domain::UNIX, Type::STREAM, None)
        .and_then(|(socket_to_auth, auth_childs_socket)| {
            socket_to_auth.set_nonblocking(true)?;
            socket_to_auth.set_cloexec(true)?;
            auth_childs_socket.set_nonblocking(true)?;

            #[cfg(target_os = "macos")] {
                socket_to_auth.set_nosigpipe(true)?;
                auth_childs_socket.set_nosigpipe(true)?;
            }

            let socket_to_auth = UnixStream::from_std(
                unsafe { StdUnixStream::from_raw_fd(socket_to_auth.into_raw_fd()) }
            );
            Ok((socket_to_auth, auth_childs_socket))
        })
        .unwrap_or_else(|e|
            error_exit!(
                "failed to create sockets for auth daemon communication: {}",
                e,
            )
        );
    let mut command = tokio::process::Command::new("dumbnotesd_auth");
    command
        .arg(format!("--socket-fd={}", auth_childs_socket.as_raw_fd()))
        .arg(path_arg("private-key-file", &config.jwt_private_key))
        .arg(path_arg("data-directory", &config.data_directory))
        .arg(path_arg("user-db-directory", &config.user_db))
        .arg(
            format!(
                "--hasher-config={}",
                serde_json::to_string(&config.hasher_config)
                    .unwrap_or_else(|e| error_exit!("cannot serialize hasher config: {e}")),
            )
        );
    let mut auth_child = command
        .spawn()
        .unwrap_or_else(|e|
            error_exit!("failed to spawn dumbnotesd_auth process: {}", e)
        );
    drop(auth_childs_socket);
    tokio::spawn(async move {
        // TODO: connect with the shutdown
        let status = auth_child.wait().await
            .unwrap_or_else(|e|
                error_exit!("waiting for dumbnotesd_auth failed: {}", e)
            );
        info!("dumbnotesd_auth child finished with {status}");
    });

    let storage: NoteStorage = NoteStorage::new(&config)
        .await
        .unwrap_or_else(|e|
            error_exit!("note storage initialization failed: {e}")
        );

    let watcher = ProductionFileWatcher::new()
        .unwrap_or_else(|e| error_exit!("failed to create file watcher: {e}"));

    let user_db: Box<dyn UserDb> = Box::new(
        ProductionUserDb::new(
            &config.user_db,
            hasher,
            watcher.clone(),
        ).await
            .unwrap_or_else(|e|
                error_exit!("could not initialize the user DB: {e}")
            )
    );

    let session_storage = Box::new(
        ProductionSessionStorage
            ::new(
                &config.data_directory,
                watcher,
            )
            .await
            .unwrap_or_else(|e|
                error_exit!("could not initialize the session DB: {e}")
            )
    );

    let jwt_private_key = read_jwt_key(&config.jwt_private_key, IsPrivate::Yes)
        .unwrap_or_else(|e|
            error_exit!("failed reading the private jwt key: {e}")
        );
    let jwt_public_key = read_jwt_key(&config.jwt_public_key, IsPrivate::No)
        .unwrap_or_else(|e|
            error_exit!("failed reading the public jwt key: {e}")
        );
    let access_token_generator = AccessTokenGenerator::from_jwk(&jwt_private_key)
        .unwrap_or_else(|e|
            error_exit!("could not initialize access token generator: {e}")
        );
    let access_token_decoder = AccessTokenDecoder::from_jwk(&jwt_public_key)
        .unwrap_or_else(|e|
            error_exit!("could not initialize access token decoder: {e}")
        );

    let access_granter = AccessGranter::new(
        session_storage,
        user_db,
        access_token_generator,
        access_token_decoder,
    );

    rocket::custom(figment)
        .manage(storage)
        .manage(config)
        .manage(access_granter)
        .install_dumbnotes_api()
        .install_dumbnotes_web()
}

gen_boolean_enum!(IsPrivate);
fn read_jwt_key(
    path: &Path,
    is_private: IsPrivate,
) -> Result<Jwk, Box<dyn Error>> {
    if is_private.into() {
        test_permissions(
            path,
            |p| p == 0o600 || p == 0o400,
            &format!(
                "{} must be owned by root and have mode of 600 or 400",
                path.to_string_lossy(),
            )
        )?;
        test_permissions(
            path.parent().expect("path has no parent"),
            |p| p & 0o022 == 0,
            &format!(
                "{} must be owned by root and not be writeable by group or other",
                path.to_string_lossy(),
            ),
        )?;
    }
    Ok(
        Jwk::from_bytes(
            std::fs::read(path)?
        )?
    )
}

#[cfg(not(debug_assertions))]
fn test_permissions(
    path: &Path,
    is_valid: impl FnOnce(u32) -> bool,
    message: &str,
) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let metadata = std::fs::metadata(path)?;
    let permissions = metadata.permissions().mode() & 0o777;
    if metadata.uid() != 0 || !is_valid(permissions) {
        error_exit!("{message}")
    }
    Ok(())
}

#[cfg(debug_assertions)]
fn test_permissions(
    _path: &Path,
    _is_valid: impl FnOnce(u32) -> bool,
    _message: &str,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}

fn path_arg(arg_name: &str, path: impl AsRef<OsStr>) -> OsString {
    let mut str = OsString::from(format!("--{arg_name}="));
    str.push(path.as_ref());
    str
}
