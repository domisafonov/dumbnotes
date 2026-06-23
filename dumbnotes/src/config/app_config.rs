use std::path::PathBuf;
use crate::config::app_config::data::AppConfigData;
use crate::config::hasher_config::ProductionHasherConfigData;

pub mod data;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub storage_user_group: Option<String>,
    pub authd_user_group: Option<String>,
    pub empty_user_group: Option<String>,
    pub data_directory: PathBuf,
    pub user_db: PathBuf,
    pub jwt_private_key: PathBuf,
    pub jwt_public_key: PathBuf,
    pub max_note_size: u64,
    pub max_note_name_size: u64,
    pub hasher_config: ProductionHasherConfigData,
    pub api_rocket_config: Option<PathBuf>,
    pub web_rocket_config: Option<PathBuf>,
    pub is_api_enabled: bool,
    pub is_web_enabled: bool,
}

impl From<AppConfigData> for AppConfig {
    fn from(value: AppConfigData) -> Self {
        AppConfig {
            storage_user_group: value.storage_user_group,
            authd_user_group: value.authd_user_group,
            empty_user_group: value.empty_user_group,
            data_directory: value.data_directory,
            user_db: value.user_db,
            jwt_private_key: value.jwt_private_key,
            jwt_public_key: value.jwt_public_key,
            max_note_size: value.max_note_size,
            max_note_name_size: value.max_note_name_size,
            hasher_config: value.hasher_config,
            api_rocket_config: value.api_rocket_config,
            web_rocket_config: value.web_rocket_config,
            is_api_enabled: value.api_enabled,
            is_web_enabled: value.web_enabled,
        }
    }
}
