#[cfg(not(debug_assertions))]
pub fn init_logging() {
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

#[cfg(debug_assertions)]
pub fn init_logging() {
    env_logger::init()
}
