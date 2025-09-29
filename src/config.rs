use std::borrow::Borrow;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::lib_constants::{DEFAULT_DATA_DIR, DEFAULT_MAX_NOTE_LEN, DEFAULT_MAX_NOTE_NAME_LEN, DEFAULT_USER_DB};

pub struct UsernameString(String);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppConfig {
    #[serde(default = "app_config_default_data_dir")]
    pub data_directory: PathBuf,

    #[serde(default = "app_config_default_user_db")]
    pub user_db: PathBuf,

    #[serde(default = "app_config_default_max_note_size")]
    pub max_note_size: u64,

    #[serde(default = "app_config_default_max_note_name_size")]
    pub max_note_name_size: u64,
}

pub fn app_config_default_data_dir() -> PathBuf {
    DEFAULT_DATA_DIR.into()
}

pub fn app_config_default_user_db() -> PathBuf {
    DEFAULT_USER_DB.into()
}

pub fn app_config_default_max_note_size() -> u64 {
    DEFAULT_MAX_NOTE_LEN
}

pub fn app_config_default_max_note_name_size() -> u64 {
    DEFAULT_MAX_NOTE_NAME_LEN
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            data_directory: DEFAULT_DATA_DIR.into(),
            user_db: DEFAULT_USER_DB.into(),
            max_note_size: DEFAULT_MAX_NOTE_LEN,
            max_note_name_size: DEFAULT_MAX_NOTE_NAME_LEN,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match() {
        assert_eq!(
            serde_json::from_str::<AppConfig>("{}").unwrap(),
            AppConfig::default()
        );
    }
}

#[derive(Debug)]
pub struct UsernameParseError;

impl std::str::FromStr for UsernameString {
    type Err = UsernameParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(UsernameString(s.to_string())) // TODO: the validation
    }
}

impl std::ops::Deref for UsernameString {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0[..]
    }
}

impl Borrow<str> for UsernameString {
    fn borrow(&self) -> &str {
        &self.0[..]
    }
}
