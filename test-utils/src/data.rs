use std::convert::Into;
use std::sync::LazyLock;
use base64ct::{Base64, Encoding};
use josekit::jwk::Jwk;

pub const MOCK_JWT_PRIVATE_KEY_STR: &str = include_str!("mock_jwt_private_key.json");
pub static MOCK_JWT_PRIVATE_KEY: LazyLock<Jwk> = LazyLock::new(||
    Jwk::from_bytes(MOCK_JWT_PRIVATE_KEY_STR)
        .expect("failed to parse mock jwt private key")
);

pub const MOCK_JWT_PUBLIC_KEY_STR: &str = include_str!("mock_jwt_public_key.json");
pub static MOCK_JWT_PUBLIC_KEY: LazyLock<Jwk> = LazyLock::new(||
    Jwk::from_bytes(MOCK_JWT_PUBLIC_KEY_STR)
        .expect("failed to parse mock jwt public key")
);

pub const MOCK_PEPPER_STR: &str = include_str!("mock_pepper.b64").trim_ascii();
pub static MOCK_PEPPER: LazyLock<Box<[u8]>> = LazyLock::new(||
    Base64::decode_vec(MOCK_PEPPER_STR)
        .expect("failed to decode mock pepper")
        .into()
);

pub const MOCK_USER_DB_STR: &str = include_str!("mock_user_db.toml");

#[cfg(test)]
mod tests {
    use std::error::Error;
    use super::*;

    #[test]
    fn mock_jwt_keys_are_coherent() -> Result<(), Box<dyn Error>> {
        assert_eq!(MOCK_JWT_PUBLIC_KEY.to_public_key()?, *MOCK_JWT_PUBLIC_KEY);
        Ok(())
    }
}
