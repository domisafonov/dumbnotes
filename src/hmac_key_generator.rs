use crate::config::app_config::AppConfig;
use josekit::jws::alg::hmac::HmacJwsAlgorithm;
use rand::TryCryptoRng;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct MakeHmacError;

pub fn make_hmac_key(
    config: &AppConfig,
    rng: &mut impl TryCryptoRng,
) -> Result<(), Box<dyn Error>> {
    let mut secret = [0u8; 64];
    rng.try_fill_bytes(&mut secret)
        .map_err(|_| MakeHmacError)?;
    fs::write(
        &config.hmac_key,
        serde_json::to_string_pretty(
            &HmacJwsAlgorithm::Hs512.to_jwk(&secret)
        )? + "\n"
    )?;
    Ok(())
}

impl Display for MakeHmacError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Error generating HMAC key")
    }
}

impl Error for MakeHmacError {}
