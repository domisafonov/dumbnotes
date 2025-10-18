use std::error::Error;
use std::fs;
use josekit::jwk::Jwk;
use josekit::jws::alg::hmac::HmacJwsAlgorithm;
use rand::Rng;
use rand::rngs::StdRng;
use crate::config::app_config::AppConfig;
use crate::rng::SyncRng;

pub fn make_hmac_key(
    config: &AppConfig,
    rng: SyncRng<StdRng>,
) -> Result<(), Box<dyn Error>> {
    fs::write(
        &config.hmac_key,
        serde_json::to_string_pretty(
            &make_jwk(rng)
        )? + "\n"
    )?;
    Ok(())
}

fn make_jwk(rng: SyncRng<StdRng>) -> Jwk {
    HmacJwsAlgorithm::Hs512.to_jwk(&rng.lock().unwrap().random::<[u8; 64]>())
}
