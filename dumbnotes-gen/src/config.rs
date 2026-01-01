use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use dumbnotes::bin_constants::{DEFAULT_DATA_DIR, DEFAULT_JWT_PRIVATE_KEY, DEFAULT_JWT_PUBLIC_KEY};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppConfig {
    #[serde(default = "app_config_default_data_dir")]
    pub data_directory: PathBuf,
    
    #[serde(default = "app_config_default_jwt_private_key")]
    pub jwt_private_key: PathBuf,

    #[serde(default = "app_config_default_jwt_public_key")]
    pub jwt_public_key: PathBuf,
}

pub fn app_config_default_data_dir() -> PathBuf {
    DEFAULT_DATA_DIR.into()
}

pub fn app_config_default_jwt_private_key() -> PathBuf {
    DEFAULT_JWT_PRIVATE_KEY.into()
}

pub fn app_config_default_jwt_public_key() -> PathBuf {
    DEFAULT_JWT_PUBLIC_KEY.into()
}
