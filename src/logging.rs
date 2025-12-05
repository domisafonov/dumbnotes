use syslog::{BasicLogger, Facility};

pub fn init_tool_logging() {
    init_logging_env()
}

pub fn init_daemon_logging(is_daemonizing: bool) {
    if is_daemonizing {
        init_logging_syslog()
    } else {
        init_tool_logging();
    }
}

fn init_logging_syslog() {
    log
        ::set_boxed_logger(
            Box::new(
                BasicLogger::new(
                    syslog::unix(
                        syslog::Formatter3164 {
                            facility: Facility::LOG_USER,
                            hostname: None,
                            process: std::env::args_os()
                                .next()
                                .map(|name|
                                    name.into_string().unwrap_or("".into())
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
    env_logger::init()
}
