pub const DEFAULT_CONFIG_FILE: &str = "/etc/dumbnotes/dumbnotes.toml";
pub const DEFAULT_USER_DB: &str = "/etc/dumbnotes/users.toml";
pub const DEFAULT_DATA_DIR: &str = "/var/dumbnotes";
pub const DEFAULT_JWT_PRIVATE_KEY: &str = "/etc/dumbnotes/private/jwt_private_key.json";
pub const DEFAULT_JWT_PUBLIC_KEY: &str = "/etc/dumbnotes/jwt_public_key.json";
pub const DEFAULT_PEPPER_PATH: &str = "/etc/dumbnotes/private/pepper.json";
pub const APP_CONFIG_ENV_PREFIX: &str = "DUMBNOTES_";

pub const IPC_MESSAGE_MAX_SIZE: usize = 1024 * 16;

pub const SESSION_ID_JWT_CLAIM_NAME: &str = "session_id";
