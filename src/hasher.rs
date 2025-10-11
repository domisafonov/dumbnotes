#[cfg(test)] mod tests;

use std::ops::DerefMut;
use argon2::{Algorithm, PasswordHash, PasswordHasher, Version};
use argon2::Argon2;
use argon2::password_hash::SaltString;
use rand::rngs::StdRng;
use crate::rng::SyncRng;

// TODO: test
// TODO: process isolation
//  this is why making the hasher async is disregarded

pub trait Hasher: Send + Sync {
    fn generate_hash(&self, password: &str) -> String;
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
    rng: SyncRng<StdRng>,
}

impl ProductionHasher {
    pub fn new(
        config: ProductionHasherConfig,
        rng: SyncRng<StdRng>,
    ) -> Self {
        ProductionHasher {
            config,
            rng,
        }
    }

    // TODO: move the parameters to the config
    // TODO: use pepper after auth process isolation is implemented
    fn get_hasher(&self) -> Argon2<'_> {
        Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            self.config.argon2_params.clone(),
        )
    }

    fn make_salt(&self) -> SaltString {
        SaltString::from_rng(self.rng.get_rng().deref_mut())
    }
}

impl Hasher for ProductionHasher {
    fn generate_hash(&self, password: &str) -> String {
        let salt = self.make_salt();
        let hasher = self.get_hasher();
        hasher.hash_password(password.as_bytes(), &salt)
            .expect("password hashing failed")
            .serialize()
            .to_string()
    }

    fn check_hash(&self, hash: PasswordHash<'_>, password: &str) -> bool {
        hash.verify_password(&[&self.get_hasher()], password)
            .map(|_| true) // TODO: log errors
            .unwrap_or(false)
    }
}
