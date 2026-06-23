use std::{fs, io};
use std::path::Path;
use thiserror::Error;
use crate::config::app_config::AppConfig;
use crate::config::app_config::data::AppConfigData;

pub fn read_app_config(
    config_file: impl AsRef<Path>,
) -> Result<AppConfig, ReadConfigError> {
    let data: AppConfigData = toml::from_slice(
        &fs::read(config_file.as_ref())?
    )?;
    Ok(data.into())
}

#[derive(Debug, Error)]
pub enum ReadConfigError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("error parsing app configuration: {0}")]
    Parse(#[from] toml::de::Error),
}
