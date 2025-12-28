use std::ffi::OsStr;
use std::path::PathBuf;
use boolean_enums::gen_boolean_enum;
use syslog::{BasicLogger, Facility};

pub fn init_tool_logging() {
    init_logging_env()
}

pub fn init_daemon_logging(is_daemonizing: IsDaemonizing) {
    if is_daemonizing.into() {
        init_logging_syslog()
    } else {
        init_tool_logging();
    }
}
gen_boolean_enum!(pub IsDaemonizing);

fn init_logging_syslog() {
    log
        ::set_boxed_logger(
            Box::new(
                BasicLogger::new(
                    syslog::unix(
                        syslog::Formatter3164 {
                            facility: Facility::LOG_USER,
                            hostname: None,
                            process: std::env::args()
                                .next()
                                .and_then(|name|
                                    PathBuf::from(name)
                                        .file_name()
                                        .map(|n|
                                            OsStr::to_string_lossy(n)
                                                .into_owned()
                                        )
                                )
                                .unwrap_or("".into()),
                            pid: std::process::id(),
                        }
                    ).expect("syslog initialization failed")
                )
            )
        )
        .map(|()| log::set_max_level(log::STATIC_MAX_LEVEL))
        .expect("syslog initialization failed");
}

fn init_logging_env() {
    env_logger::builder()
        .filter_level(
            if cfg!(debug_assertions) {
                log::LevelFilter::Debug
            } else {
                log::LevelFilter::Info
            }
        )
        .init()
}
