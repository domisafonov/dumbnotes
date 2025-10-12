use std::path::Path;
use rocket::figment::Figment;
use rocket::figment::providers::{Env, Format, Serialized, Toml};
use crate::bin_constants::APP_CONFIG_ENV_PREFIX;
use crate::config::app_config::AppConfig;

pub trait FigmentExt {
    fn setup_app_config(
        self,
        config_file: impl AsRef<Path>,
    ) -> Figment;
}

impl FigmentExt for Figment {
    fn setup_app_config(self, config_file: impl AsRef<Path>) -> Figment {
        // TODO: error if unknown keys are in the config file
        self.merge(Serialized::defaults(AppConfig::default()))
            .merge(Toml::file_exact(config_file))
            .merge(Env::prefixed(APP_CONFIG_ENV_PREFIX).global())
    }
}
