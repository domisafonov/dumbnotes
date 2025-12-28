use time::Duration;

// relative to the data directory
pub const SESSION_STORAGE_PATH: &str = "private/session.toml";
pub const SESSION_STORAGE_READ_BUF_SIZE: usize = 1024 * 128;
pub const REFRESH_TOKEN_SIZE: usize = 128 / 8;
pub const REFRESH_TOKEN_GC_TIME: Duration = Duration::weeks(5);
pub const ACCESS_TOKEN_VALIDITY_TIME: Duration = Duration::minutes(15);

pub const FILE_WATCHER_DEBOUNCE_TIME: Duration = Duration::seconds(10);
