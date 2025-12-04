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
    use syslog::BasicLogger;

    log
        ::set_boxed_logger(
            Box::new(
                BasicLogger::new(
                    syslog::unix(
                        // for some reason, only 3164 has log crate
                        // integration at the moment
                        syslog::Formatter3164::default(),
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
