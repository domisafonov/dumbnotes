use crate::access_granter::{AccessGranter, ProductionAccessGranter};
use crate::routes::{ApiRocketBuildExt, WebRocketBuildExt};
use async_trait::async_trait;
use dumbnotes::access_token::AccessTokenDecoder;
use dumbnotes::config::app_config::AppConfig;
use dumbnotes::error_exit;
use dumbnotes::ipc::socket::create_socket_pair;
#[cfg(target_os = "openbsd")] use dumbnotes::sandbox::pledge::pledge_liftoff;
use dumbnotes::storage::NoteStorage;
use josekit::jwk::Jwk;
use log::{error, info};
use rocket::fairing::{Fairing, Info};
use rocket::{Build, Orbit, Rocket};
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::io;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::sync::Arc;
use tokio::net::UnixStream;
use tokio::sync::{oneshot, Mutex};
use dumbnotes::sandbox::user_group::{clear_supplementary_groups, get_user_and_group, set_user_and_group};

pub struct AppSetupFairing {
    is_daemonizing: bool,
    auth_daemon_failure_notice: Arc<Mutex<Option<oneshot::Receiver<()>>>>,
}

impl AppSetupFairing {
    pub fn new(is_daemonizing: bool) -> Self {
        AppSetupFairing {
            is_daemonizing,
            auth_daemon_failure_notice: Arc::new(Mutex::new(None)),
        }
    }

    async fn launch_authd(&self, config: &AppConfig) -> Result<UnixStream, Box<dyn Error>> {
        let (socket_to_auth, auth_childs_socket) = create_socket_pair()
            .inspect_err(|e|
                error!(
                    "failed to create sockets for auth daemon communication: {}",
                    e,
                )
            )?;

        let mut command = tokio::process::Command::new("dumbnotesd_auth");
        command
            .arg(format!("--socket-fd={}", auth_childs_socket.as_raw_fd()))
            .arg(path_arg("private-key-file", &config.jwt_private_key))
            .arg(path_arg("data-directory", &config.data_directory))
            .arg(path_arg("user-db-path", &config.user_db))
            .arg(
                format!(
                    "--hasher-config={}",
                    serde_json::to_string(&config.hasher_config)
                        .inspect_err(|e|
                            error!("cannot serialize hasher config: {e}")
                        )?,
                )
            );
        if self.is_daemonizing {
            if cfg!(debug_assertions) {
                command.arg("--daemonize");
            } else {
                command.arg("--no-daemonize");
            }

            if let Some(ref authd_user_group) = config.authd_user_group {
                let (uid, gid) = get_user_and_group(authd_user_group)?;
                command.uid(uid).gid(gid);
            }
        }
        let mut auth_child = command.spawn()
            .inspect_err(|e|
                error!("failed to spawn dumbnotesd_auth process: {}", e)
            )?;
        drop(auth_childs_socket);

        let (auth_failed_sender, auth_failed_receiver) = oneshot::channel();
        *self.auth_daemon_failure_notice.lock().await = Some(auth_failed_receiver);
        tokio::spawn(async move {
            let status = auth_child.wait().await;
            let send_result = match status {
                Ok(status) => {
                    info!("dumbnotesd_auth child finished with {status}");
                    auth_failed_sender.send(())
                },
                Err(e) => {
                    error!("waiting for dumbnotesd_auth failed: {}", e);
                    auth_failed_sender.send(())
                }
            };
            if send_result.is_err() {
                error_exit!("failed to initiate shutdown, terminating immediately")
            }
        });
        Ok(socket_to_auth)
    }
}

macro_rules! ok_or_bail {
    ($rocket:ident, $expr:expr, |$e:ident| $error_logger:expr) => ({
        match $expr {
            std::result::Result::Ok(ok) => ok,
            std::result::Result::Err(e) => {
                let $e = e;
                $error_logger;
                return std::result::Result::Err($rocket);
            },
        }
    });
}

#[async_trait]
impl Fairing for AppSetupFairing {
    fn info(&self) -> Info {
        use rocket::fairing::Kind;
        Info {
            name: "app setup",
            kind: Kind::Ignite | Kind::Liftoff,
        }
    }

    async fn on_ignite(
        &self,
        rocket: Rocket<Build>,
    ) -> rocket::fairing::Result {
        let config: AppConfig = ok_or_bail!(
            rocket,
            rocket.figment().extract(),
            |e| {
                for e in e {
                    error!("{e}");
                }
                info!("finishing due to a config parse error");
            }
        );

        ok_or_bail!(
            rocket,
            clear_supplementary_groups(),
            |e| error!("failed to clear up supplementary groups: {e}")
        );

        let socket_to_auth = ok_or_bail!(
            rocket,
            self.launch_authd(&config).await,
            |e| error!("failed to launch dumbnotesd_auth: {e}")
        );

        if self.is_daemonizing && let Some(ref user_group) = config.user_group {
            ok_or_bail!(
                rocket,
                set_user_and_group(user_group),
                |e| error!("failed to set user and group: {e}")
            )
        }

        let storage: NoteStorage = ok_or_bail!(
            rocket,
            NoteStorage::new(&config).await,
            |e| error!("note storage initialization failed: {e}")
        );

        let jwt_public_key = ok_or_bail!(
            rocket,
            read_jwt_key(&config.jwt_public_key),
            |e| error!("failed reading the public jwt key: {e}")
        );
        let access_token_decoder = ok_or_bail!(
            rocket,
            AccessTokenDecoder::from_jwk(&jwt_public_key),
            |e| error!("could not initialize access token decoder: {e}")
        );

        let access_granter: Box<dyn AccessGranter> = Box::new(
            ProductionAccessGranter::new(
                access_token_decoder,
                socket_to_auth
            ).await
        );

        Ok(
            rocket
                .manage(storage)
                .manage(config)
                .manage(access_granter)
                .install_dumbnotes_api()
                .install_dumbnotes_web()
        )
    }

    async fn on_liftoff(
        &self,
        rocket: &Rocket<Orbit>,
    ) {
        #[cfg(target_os = "openbsd")] pledge_liftoff();
        let shutdown = rocket.shutdown();
        let auth_daemon_failure_notice = self.auth_daemon_failure_notice.clone();
        tokio::spawn(async move {
            let receiver = auth_daemon_failure_notice.lock().await.take()
                .unwrap_or_else(||
                    error_exit!("failed to initiate graceful shutdown, terminating immediately")
                );
            receiver.await
                .unwrap_or_else(|e|
                    error_exit!("failed to initiate graceful shutdown terminating immediately: {e}")
                );
            shutdown.notify();
        });
    }
}

fn read_jwt_key(path: &Path) -> Result<Jwk, Box<dyn Error>> {
    Ok(Jwk::from_bytes(std::fs::read(path)?)?)
}

fn path_arg(arg_name: &str, path: impl AsRef<OsStr>) -> OsString {
    let mut str = OsString::from(format!("--{arg_name}="));
    str.push(path.as_ref());
    str
}
