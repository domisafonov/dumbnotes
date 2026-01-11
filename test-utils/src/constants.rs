use std::time::Duration;

pub const TERM_WAIT: Duration = Duration::from_millis(5000);
pub const KILL_CHECK_INTERVAL: Duration = Duration::from_millis(100);
pub const BACKGROUND_READER_CHECK_INTERVAL: Duration = Duration::from_millis(100);