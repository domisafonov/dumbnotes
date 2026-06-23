use crate::lib_constants::{DEFAULT_MAX_NOTE_LEN, DEFAULT_MAX_NOTE_NAME_LEN};

pub const DEFAULT_CONFIG_FILE: &str = "/etc/dumbnotes/dumbnotes.toml";
pub const DEFAULT_USER_DB: &str = "/etc/dumbnotes/private/users.toml";
pub const DEFAULT_DATA_DIR: &str = "/var/dumbnotes";
pub const DEFAULT_JWT_PRIVATE_KEY: &str = "/etc/dumbnotes/private/jwt_private_key.json";
pub const DEFAULT_JWT_PUBLIC_KEY: &str = "/etc/dumbnotes/jwt_public_key.json";
pub const DEFAULT_PEPPER_PATH: &str = "/etc/dumbnotes/private/pepper.b64";
pub const PEPPER_LENGTH: usize = 128 / 8;
pub const APP_CONFIG_API_ENV_PREFIX: &str = "DUMBNOTES_API_";
pub const APP_CONFIG_WEB_ENV_PREFIX: &str = "DUMBNOTES_ENV_";

pub const IPC_MESSAGE_MAX_SIZE: usize = 1024 * 16;
pub const IPC_STORAGE_MESSAGE_MAX_SIZE: usize = (DEFAULT_MAX_NOTE_LEN as usize + DEFAULT_MAX_NOTE_NAME_LEN as usize) * 2; // TODO: validate

pub const SESSION_ID_JWT_CLAIM_NAME: &str = "session_id";
