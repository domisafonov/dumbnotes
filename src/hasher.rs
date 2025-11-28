#[cfg(test)] mod tests;

use argon2::password_hash::SaltString;
use argon2::Argon2;
use argon2::{Algorithm, PasswordHash, PasswordHasher, Version};
use log::error;
use rand::rand_core::OsError;
use rand::rngs::OsRng;

pub trait Hasher: Send + Sync {
    fn generate_hash(&self, password: &str) -> Result<String, OsError>;
    fn check_hash(&self, hash: PasswordHash<'_>, password: &str) -> bool;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionHasherConfig {
    pub argon2_params: argon2::Params,
}

impl ProductionHasherConfig {
    pub fn new(argon2_params: argon2::Params) -> Self {
        ProductionHasherConfig { 
            argon2_params 
        }
    }
}

pub struct ProductionHasher {
    config: ProductionHasherConfig,
}

impl ProductionHasher {
    pub fn new(
        config: ProductionHasherConfig,
    ) -> Self {
        ProductionHasher {
            config,
        }
    }

    fn get_hasher(&self) -> Argon2<'_> {
        Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            self.config.argon2_params.clone(),
        )
    }

    fn make_salt(&self) -> Result<SaltString, OsError> {
        SaltString::try_from_rng(&mut OsRng)
    }
}

impl Hasher for ProductionHasher {
    fn generate_hash(&self, password: &str) -> Result<String, OsError> {
        let salt = self.make_salt()?;
        let hasher = self.get_hasher();
        Ok(
            hasher.hash_password(password.as_bytes(), &salt)
                .expect("password hashing failed")
                .serialize()
                .to_string()
        )
    }

    fn check_hash(&self, hash: PasswordHash<'_>, password: &str) -> bool {
        hash.verify_password(&[&self.get_hasher()], password)
            .map(|_| true)
            .or_else(|e|
                if let argon2::password_hash::Error::Password = e {
                    Ok(false)
                } else {
                    Err(e)
                }
            )
            .unwrap_or_else(|e| {
                error!("failed to check password hash: {e}");
                false
            })
    }
}
