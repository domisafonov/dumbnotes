use crate::access_granter::{AccessGranter, ProductionAccessGranter};
use crate::routes::{ApiRocketBuildExt, WebRocketBuildExt};
use crate::storage_accessor::{ProductionStorageAccessor, StorageAccessor};
use async_trait::async_trait;
use dumbnotes::access_token::AccessTokenDecoder;
use dumbnotes::config::app_config::AppConfig;
use futures::FutureExt;
use futures::future::{join_all, select_all};
use tokio::process::Command;
use util::error_exit;
use dumbnotes::ipc::socket::create_socket_pairs;
#[cfg(target_os = "openbsd")] use dumbnotes::sandbox::pledge::pledge_liftoff;
#[cfg(target_os = "openbsd")] use dumbnotes::sandbox::unveil::{Permissions, unveil, seal_unveil};
use josekit::jwk::Jwk;
use log::{error, info};
use rocket::fairing::{Fairing, Info};
use rocket::{Build, Orbit, Rocket};
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::os::fd::{AsRawFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use boolean_enums::gen_boolean_enum;
use tokio::sync::{oneshot, Mutex};
use dumbnotes::sandbox::user_group::{clear_supplementary_groups, get_user_and_group, set_user_and_group};

pub struct AppSetupFairing {
    app_config: AppConfig,
    is_daemonizing: bool,
    authd_path: PathBuf,
    storaged_path: PathBuf,
    auth_daemon_failure_notice: Arc<Mutex<Option<oneshot::Receiver<()>>>>,
    storage_daemon_failure_notice: Arc<Mutex<Option<oneshot::Receiver<()>>>>,
    temp_dir: PathBuf,
}

impl AppSetupFairing {
    pub fn new(
        app_config: AppConfig,
        is_daemonizing: IsDaemonizing,
        authd_path: impl ToOwned<Owned=PathBuf>,
        storaged_path: impl ToOwned<Owned=PathBuf>,
        temp_dir: impl ToOwned<Owned=PathBuf>,
    ) -> Self {
        AppSetupFairing {
            app_config,
            is_daemonizing: is_daemonizing.into(),
            authd_path: authd_path.to_owned(),
            storaged_path: storaged_path.to_owned(),
            auth_daemon_failure_notice: Arc::new(Mutex::new(None)),
            storage_daemon_failure_notice: Arc::new(Mutex::new(None)),
            temp_dir: temp_dir.to_owned(),
        }
    }

    async fn launch_storaged(
        &self,
        storage_childs_socket: OwnedFd,
    ) -> Result<(), Box<dyn Error>> {
        let mut command = Command::new(&self.storaged_path);
        command
            .arg(path_arg("public-key-file", &self.app_config.jwt_public_key))
            .arg(path_arg("data-directory", &self.app_config.data_directory))
            .arg(format!("--max-note-len={}", self.app_config.max_note_size))
            .arg(format!("--max-note-name-len={}", self.app_config.max_note_name_size));
        self.launch_sub(
            self.storaged_path.file_name().unwrap().to_string_lossy(),
            command,
            None, // TODO
            storage_childs_socket,
            self.storage_daemon_failure_notice.clone(),
        ).await
    }

    async fn launch_authd(
        &self,
        auth_childs_socket: OwnedFd,
    ) -> Result<(), Box<dyn Error>> {
        let mut command = Command::new(&self.authd_path);
        command
            .arg(path_arg("private-key-file", &self.app_config.jwt_private_key))
            .arg(path_arg("data-directory", &self.app_config.data_directory))
            .arg(path_arg("user-db-path", &self.app_config.user_db))
            .arg(
                format!(
                    "--hasher-config={}",
                    serde_json::to_string(&self.app_config.hasher_config)
                        .inspect_err(|e|
                            error!("cannot serialize hasher config: {e}")
                        )?,
                )
            );
        self.launch_sub(
            self.authd_path.file_name().unwrap().to_string_lossy(),
            command,
            self.app_config.authd_user_group.as_ref().map(String::as_str),
            auth_childs_socket,
            self.auth_daemon_failure_notice.clone(),
        ).await
    }

    async fn launch_sub(
        &self,
        daemon_name: impl AsRef<str>,
        mut command: Command,
        user_group: Option<&str>,
        childs_socket: OwnedFd,
        failure_notice: Arc<Mutex<Option<oneshot::Receiver<()>>>>,
    ) -> Result<(), Box<dyn Error>> {
        command.arg(format!("--socket-fds={}", childs_socket.as_raw_fd()));
        if self.is_daemonizing && cfg!(debug_assertions) {
            command.arg("--daemonize");
        }
        if !self.is_daemonizing && !cfg!(debug_assertions) {
            command.arg("--no-daemonize");
        }
        if self.is_daemonizing && let Some(user_group) = user_group
        {
            let (uid, gid) = get_user_and_group(user_group)?;
            command.uid(uid).gid(gid);
        }
        let mut child = command.spawn()
            .inspect_err(|e|
                error!("failed to spawn dumbnotesd-auth process: {}", e)
            )?;
        drop(childs_socket);

        let (failed_sender, failed_receiver) = oneshot::channel();
        *failure_notice.lock().await = Some(failed_receiver);
        let daemon_name = daemon_name.as_ref().to_owned();
        tokio::spawn(async move {
            let status = child.wait().await;
            let send_result = match status {
                Ok(status) => {
                    info!("{daemon_name} child finished with {status}");
                    failed_sender.send(())
                },
                Err(e) => {
                    error!("waiting for {daemon_name} failed: {}", e);
                    failed_sender.send(())
                }
            };
            if send_result.is_err() {
                error_exit!("failed to initiate shutdown, terminating immediately")
            }
        });

        Ok(())
    }
}

macro_rules! ok_or_bail {
    ($rocket:ident, $expr:expr, |$e:ident| $error_logger:expr$(,)*) => ({
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
        if self.is_daemonizing {
            ok_or_bail!(
                rocket,
                clear_supplementary_groups(),
                |e| error!("failed to clear up supplementary groups: {e}")
            );
        }

        let mut sockets = ok_or_bail!(
            rocket,
            create_socket_pairs(2),
            |e| error!("failed to create sockets for IPC: {e}"),
        ).into_iter();

        let (socket_to_storage, storage_childs_socket) = sockets.next().unwrap();
        ok_or_bail!(
            rocket,
            self.launch_storaged(storage_childs_socket).await,
            |e| error!("failed to launch dumbnotesd-storage: {e}"),
        );
        let (socket_to_auth, auth_childs_socket) = sockets.next().unwrap();
        ok_or_bail!(
            rocket,
            self.launch_authd(auth_childs_socket).await,
            |e| error!("failed to launch dumbnotesd-auth: {e}"),
        );
        assert!(sockets.next().is_none());

        #[cfg(target_os = "openbsd")] {
            unveil(
                &self.app_config.jwt_public_key,
                Permissions::R,
            );
            unveil(
                &self.temp_dir,
                Permissions::C | Permissions::R | Permissions::W,
            );
            seal_unveil()
        }

        if self.is_daemonizing
            && let Some(ref user_group) = self.app_config.user_group
        {
            ok_or_bail!(
                rocket,
                set_user_and_group(user_group),
                |e| error!("failed to set user and group: {e}")
            )
        }

        let storage_accessor: Box<dyn StorageAccessor> = Box::new(
            ProductionStorageAccessor::new(socket_to_storage).await
        );
        let jwt_public_key = ok_or_bail!(
            rocket,
            read_jwt_key(&self.app_config.jwt_public_key),
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
                .manage(storage_accessor)
                .manage(self.app_config.clone())
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
        let failure_notices = vec![
            self.auth_daemon_failure_notice.clone(),
            self.storage_daemon_failure_notice.clone(),
        ];
        tokio::spawn(async move {
            let receivers = join_all(
                failure_notices
                    .into_iter()
                    .map(|n|
                        n.lock_owned()
                            .map(|mut r| r.take().unwrap_or_else(||
                                error_exit!("failed to initiate graceful shutdown, terminating immediately")
                            ))
                    )
                    .collect::<Vec<_>>()
            ).await;

            let _ = select_all(receivers).await;
            shutdown.notify();
        });
    }
}
gen_boolean_enum!(pub IsDaemonizing);

fn read_jwt_key(path: &Path) -> Result<Jwk, Box<dyn Error>> {
    Ok(Jwk::from_bytes(std::fs::read(path)?)?)
}

fn path_arg(arg_name: &str, path: impl AsRef<OsStr>) -> OsString {
    let mut str = OsString::from(format!("--{arg_name}="));
    str.push(path.as_ref());
    str
}
