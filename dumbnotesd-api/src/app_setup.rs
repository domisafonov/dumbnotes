use crate::access_granter::{AccessGranter, ProductionAccessGranter};
use crate::routes::ApiRocketBuildExt;
use futures::FutureExt;
use futures::future::{join_all, select_all};
use storage_ipc_sdk::{ProductionStorageAccessor, StorageAccessor};
use async_trait::async_trait;
use dumbnotes::access_token::{AccessTokenDecoder, AccessTokenValidator};
use dumbnotes::ipc::socket::discover_socket;
#[cfg(target_os = "openbsd")] use dumbnotes::sandbox::pledge::pledge_apid_liftoff;
#[cfg(target_os = "openbsd")] use dumbnotes::sandbox::unveil::{Permissions, unveil, seal_unveil};
use josekit::jwk::Jwk;
use log::error;
use rocket::fairing::{Fairing, Info};
use rocket::{Build, Orbit, Rocket};
use tokio::sync::{Mutex, oneshot};
use util::error_exit;
use std::error::Error;
use std::os::fd::RawFd;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct AppSetupFairing {
    jwt_public_key: PathBuf,
    auth_socket_fd: RawFd,
    storage_socket_fd: RawFd,
    temp_dir: PathBuf,
    auth_daemon_failure_notice: Arc<Mutex<Option<oneshot::Receiver<()>>>>,
    storage_daemon_failure_notice: Arc<Mutex<Option<oneshot::Receiver<()>>>>,
}

impl AppSetupFairing {
    pub fn new(
        jwt_public_key: PathBuf,
        auth_socket_fd: RawFd,
        storage_socket_fd: RawFd,
        temp_dir: impl ToOwned<Owned=PathBuf>,
    ) -> Self {
        AppSetupFairing {
            jwt_public_key,
            auth_socket_fd,
            storage_socket_fd,
            temp_dir: temp_dir.to_owned(),
            auth_daemon_failure_notice: Arc::new(Mutex::new(None)),
            storage_daemon_failure_notice: Arc::new(Mutex::new(None)),
        }
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
        let auth_socket = discover_socket(self.auth_socket_fd);
        let storage_socket = discover_socket(self.storage_socket_fd);

        #[cfg(target_os = "openbsd")] {
            unveil(
                &self.jwt_public_key,
                Permissions::R,
            );
            unveil(
                &self.temp_dir,
                Permissions::C | Permissions::R | Permissions::W,
            );
            seal_unveil()
        }

        let (storage_accessor, storage_accessor_shutdown_notice) =
            ProductionStorageAccessor::new(storage_socket).await;
        *self.storage_daemon_failure_notice.lock().await =
            Some(storage_accessor_shutdown_notice);
        let storage_accessor: Box<dyn StorageAccessor> = Box::new(storage_accessor);
        let jwt_public_key = ok_or_bail!(
            rocket,
            read_jwt_key(&self.jwt_public_key),
            |e| error!("failed reading the public jwt key: {e}")
        );
        let access_token_decoder = ok_or_bail!(
            rocket,
            AccessTokenDecoder::from_jwk(&jwt_public_key),
            |e| error!("could not initialize access token decoder: {e}")
        );
        let access_token_validator = AccessTokenValidator::new(
            access_token_decoder,
        );

        let (access_granter, access_granter_shutdown_notice) =
            ProductionAccessGranter::new(
                access_token_validator,
                auth_socket,
            ).await;
        *self.auth_daemon_failure_notice.lock().await =
            Some(access_granter_shutdown_notice);
        let access_granter: Box<dyn AccessGranter> = Box::new(access_granter);

        // FIXME: chroot into the tmp directory (fix the dir in main.rs too)

        Ok(
            rocket
                .manage(storage_accessor)
                .manage(access_granter)
                .install_dumbnotes_api()
        )
    }

    async fn on_liftoff(
        &self,
        rocket: &Rocket<Orbit>,
    ) {
        #[cfg(target_os = "openbsd")] pledge_apid_liftoff();
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

fn read_jwt_key(path: &Path) -> Result<Jwk, Box<dyn Error>> {
    Ok(Jwk::from_bytes(std::fs::read(path)?)?)
}
