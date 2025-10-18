pub const DEFAULT_CONFIG_FILE: &str = "/etc/dumbnotes/dumbnotes.toml";
pub const DEFAULT_USER_DB: &str = "/etc/dumbnotes/users.toml";
pub const DEFAULT_DATA_DIR: &str = "/var/dumbnotes";
pub const DEFAULT_HMAC_KEY: &str = "/etc/dumbnotes/hmac_key.json";
pub const APP_CONFIG_ENV_PREFIX: &str = "DUMBNOTES_";

// the defaults are taken from the argon2 crate itself
// TODO: check that the defaults are sane
pub const DEFAULT_ARGON2_M_COST: u32 = 19 * 1024;
pub const DEFAULT_ARGON2_T_COST: u32 = 2;
pub const DEFAULT_ARGON2_P_COST: u32 = 1;
pub const DEFAULT_ARGON2_OUTPUT_LEN: Option<usize> = Some(32);
