#[cfg(test)] mod tests;

use std::error::Error;
use std::{fs, io};
use std::path::PathBuf;
use argon2::password_hash::SaltString;
use argon2::Argon2;
use argon2::{Algorithm, PasswordHash, PasswordHasher, Version};
use base64ct::{Base64, Encoding};
use rand::rand_core::OsError;
use rand::rngs::OsRng;
use thiserror::Error;

pub trait Hasher: Send + Sync {
    fn generate_hash(&self, password: &str) -> Result<String, HasherError>;
    fn check_hash(
        &self,
        hash: PasswordHash<'_>,
        password: &str,
    ) -> Result<bool, HasherError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionHasherConfig {
    pub argon2_params: argon2::Params,
    pub pepper: PathBuf,
}

impl ProductionHasherConfig {
    pub fn new(
        argon2_params: argon2::Params,
        pepper: PathBuf,
    ) -> Self {
        ProductionHasherConfig {
            argon2_params,
            pepper,
        }
    }
}

pub struct ProductionHasher {
    argon2_params: argon2::Params,
    pepper: Box<[u8]>,
}

impl ProductionHasher {
    pub fn new(
        config: ProductionHasherConfig,
    ) -> Result<Self, HasherError> {
        let ret = ProductionHasher {
            argon2_params: config.argon2_params,
            pepper: Base64
                ::decode_vec(
                    fs::read_to_string(config.pepper)?
                        .trim_ascii_end()
                )
                .map_err(HasherError::PepperDecode)?
                .into(),
        };
        ret.get_hasher()
            .map_err(|e| HasherError::Initialization(Box::new(e)))?;
        Ok(ret)
    }

    fn get_hasher(&self) -> Result<Argon2<'_>, argon2::Error> {
        Argon2::new_with_secret(
            &self.pepper,
            Algorithm::Argon2id,
            Version::V0x13,
            self.argon2_params.clone(),
        )
    }

    fn make_salt(&self) -> Result<SaltString, OsError> {
        SaltString::try_from_rng(&mut OsRng)
    }
}

impl Hasher for ProductionHasher {
    fn generate_hash(&self, password: &str) -> Result<String, HasherError> {
        let salt = self.make_salt()?;
        let hasher = self.get_hasher().expect("failed to initialize argon2");
        hasher.hash_password(password.as_bytes(), &salt)
            .map_err(|e| HasherError::Hash(Box::new(e)))
            .map(|v| v.serialize().to_string())
    }

    fn check_hash(&self, hash: PasswordHash<'_>, password: &str) -> Result<bool, HasherError> {
        hash
            .verify_password(
                &[&self.get_hasher().expect("failed to initialize argon2")],
                password,
            )
            .map(|_| true)
            .or_else(|e|
                if let argon2::password_hash::Error::Password = e {
                    Ok(false)
                } else {
                    Err(e)
                }
            )
            .map_err(|e| HasherError::Hash(Box::new(e)))
    }
}

#[derive(Debug, Error)]
pub enum HasherError {
    #[error("failed to initialize password hasher: {0}")]
    Initialization(Box<dyn Error + Send + Sync>),

    #[error("failed to hash password: {0}")]
    Hash(Box<dyn Error + Send + Sync>),

    #[error("failed to get random values: {0}")]
    Random(#[from] OsError),

    #[error("failed to decode pepper: {0}")]
    PepperDecode(base64ct::Error),

    #[error(transparent)]
    Io(#[from] io::Error),
}
