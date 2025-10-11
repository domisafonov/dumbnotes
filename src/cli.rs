use clap::Args;

// the defaults are taken from the argon2 crate itself
// TODO: check that the defaults are sane
#[derive(Args, Clone, Debug, Eq, PartialEq)]
pub struct ProductionHasherConfig {
    #[arg(long, default_value_t = 19 * 1024)]
    pub argon2_m_cost: u32,

    #[arg(long, default_value_t = 2)]
    pub argon2_t_cost: u32,

    #[arg(long, default_value_t = 1)]
    pub argon2_p_cost: u32,

    #[arg(long, default_value = "32")]
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
