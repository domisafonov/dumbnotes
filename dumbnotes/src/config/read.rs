use std::{fs, io};
use std::path::Path;
use figment::Figment;
use figment::providers::{Env, Serialized};
use thiserror::Error;
use crate::bin_constants::APP_CONFIG_ENV_PREFIX;
use crate::config::app_config::AppConfig;
use crate::config::app_config::data::AppConfigData;

pub fn read_app_config(
    config_file: impl AsRef<Path>,
    rocket_defaults: Figment,
) -> Result<ReadConfig, ReadConfigError> {
    let mut data: AppConfigData = toml::from_slice(
        &fs::read(config_file.as_ref())?
    )?;
    let rocket_figment = match data.rocket.take() {
        Some(config) => rocket_defaults.merge(
            Serialized::defaults(config)
        ),
        None => rocket_defaults,
    };
    let rocket_figment = rocket_figment
        .merge(Env::prefixed(APP_CONFIG_ENV_PREFIX).global());
    Ok(
        ReadConfig {
            app_config: data.into(),
            rocket_figment,
        }
    )
}

#[derive(Debug)]
pub struct ReadConfig {
    pub app_config: AppConfig,
    pub rocket_figment: Figment,
}

#[derive(Debug, Error)]
pub enum ReadConfigError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("error parsing app configuration: {0}")]
    Parse(#[from] toml::de::Error),
}
