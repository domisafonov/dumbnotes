use time::Duration;

// relative to the data directory
pub const SESSION_STORAGE_PATH: &str = "private/session.toml";
pub const SESSION_STORAGE_READ_BUF_SIZE: usize = 1024 * 128;
pub const REFRESH_TOKEN_SIZE: usize = 128 / 8;
pub const XSRF_TOKEN_SIZE: usize = 128 / 8;
pub const REFRESH_TOKEN_VALIDITY_TIME: Duration = Duration::weeks(5);
pub const API_ACCESS_TOKEN_VALIDITY_TIME: Duration = Duration::minutes(15);
pub const WEB_ACCESS_TOKEN_VALIDITY_TIME: Duration = Duration::weeks(5);

pub const FILE_WATCHER_DEBOUNCE_TIME: Duration = Duration::seconds(10);

pub const SHUTDOWN_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(1200);
