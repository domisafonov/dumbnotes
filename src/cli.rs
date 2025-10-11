use clap::Args;
use crate::app_constants::{DEFAULT_ARGON2_M_COST, DEFAULT_ARGON2_OUTPUT_LEN, DEFAULT_ARGON2_P_COST, DEFAULT_ARGON2_T_COST};

#[derive(Args, Clone, Debug, Eq, PartialEq)]
pub struct ProductionHasherConfig {
    #[arg(long, default_value_t = DEFAULT_ARGON2_M_COST)]
    pub argon2_m_cost: u32,

    #[arg(long, default_value_t = DEFAULT_ARGON2_T_COST)]
    pub argon2_t_cost: u32,

    #[arg(long, default_value_t = DEFAULT_ARGON2_P_COST)]
    pub argon2_p_cost: u32,

    #[arg(long, default_value = DEFAULT_ARGON2_OUTPUT_LEN)]
    pub argon2_output_len: Option<usize>,
}

impl TryFrom<ProductionHasherConfig> for argon2::Params {
    type Error = argon2::Error;
    fn try_from(value: ProductionHasherConfig) -> Result<Self, Self::Error> {
        argon2::Params::new(
            value.argon2_m_cost,
            value.argon2_p_cost,
            value.argon2_t_cost,
            value.argon2_output_len,
        )
    }
}
