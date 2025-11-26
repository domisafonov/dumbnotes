use time::Duration;

pub const WEB_PREFIX: &str = "/web";
pub const API_PREFIX: &str = "/api";

pub const API_VERSION: &str = "1";

pub const DEFAULT_PROTOBUF_READ_LIMIT: u64 = 1024 * 1024;

pub const ACCESS_TOKEN_VALIDITY_TIME: Duration = Duration::minutes(15);
