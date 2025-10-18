use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use dumbnotes::bin_constants::{DEFAULT_DATA_DIR, DEFAULT_HMAC_KEY};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppConfig {
    #[serde(default = "app_config_default_data_dir")]
    pub data_directory: PathBuf,
    
    #[serde(default = "app_config_default_hmac_key")]
    pub hmac_key: PathBuf,
}

pub fn app_config_default_data_dir() -> PathBuf {
    DEFAULT_DATA_DIR.into()
}

pub fn app_config_default_hmac_key() -> PathBuf {
    DEFAULT_HMAC_KEY.into()
}