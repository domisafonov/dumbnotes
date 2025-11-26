use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::bin_constants::DEFAULT_PEPPER_PATH;
use crate::lib_constants::{DEFAULT_ARGON2_M_COST, DEFAULT_ARGON2_OUTPUT_LEN, DEFAULT_ARGON2_P_COST, DEFAULT_ARGON2_T_COST};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProductionHasherConfigData {
    #[serde(default = "production_hasher_config_default_argon2_m_cost")]
    pub argon2_m_cost: u32,

    #[serde(default = "production_hasher_config_default_argon2_t_cost")]
    pub argon2_t_cost: u32,

    #[serde(default = "production_hasher_config_default_argon2_p_cost")]
    pub argon2_p_cost: u32,

    #[serde(default = "production_hasher_config_default_argon2_output_len")]
    pub argon2_output_len: Option<usize>,

    // TODO: actually use
    #[serde(default = "production_hasher_config_default_pepper_path")]
    pub pepper_path: PathBuf,
}

pub fn production_hasher_config_default_argon2_m_cost() -> u32 {
    DEFAULT_ARGON2_M_COST
}

pub fn production_hasher_config_default_argon2_t_cost() -> u32 {
    DEFAULT_ARGON2_T_COST
}

pub fn production_hasher_config_default_argon2_p_cost() -> u32 {
    DEFAULT_ARGON2_P_COST
}

pub fn production_hasher_config_default_argon2_output_len() -> Option<usize> {
    DEFAULT_ARGON2_OUTPUT_LEN
}

pub fn production_hasher_config_default_pepper_path() -> PathBuf {
    DEFAULT_PEPPER_PATH.into()
}

impl TryFrom<ProductionHasherConfigData> for argon2::Params {
    type Error = argon2::Error;
    fn try_from(value: ProductionHasherConfigData) -> Result<Self, Self::Error> {
        argon2::Params::new(
            value.argon2_m_cost,
            value.argon2_p_cost,
            value.argon2_t_cost,
            value.argon2_output_len,
        )
    }
}

impl Default for ProductionHasherConfigData {
    fn default() -> Self {
        ProductionHasherConfigData {
            argon2_m_cost: DEFAULT_ARGON2_M_COST,
            argon2_t_cost: DEFAULT_ARGON2_T_COST,
            argon2_p_cost: DEFAULT_ARGON2_P_COST,
            argon2_output_len: DEFAULT_ARGON2_OUTPUT_LEN,
            pepper_path: DEFAULT_PEPPER_PATH.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hasher_config_defaults_match() {
        assert_eq!(
            ProductionHasherConfigData::default(),
            serde_json::de::from_str("{}").unwrap(),
        )
    }
}
