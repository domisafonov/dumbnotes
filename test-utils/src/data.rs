use std::convert::Into;
use std::sync::LazyLock;
use base64ct::{Base64, Encoding};
use josekit::jwk::Jwk;

pub const MOCK_JWT_PRIVATE_KEY_STR: &str = r#"{
  "kty": "OKP",
  "use": "sig",
  "crv": "Ed25519",
  "d": "vtjM6IpOUep3coiNOQFC7AY7YeA8NwdAvlXUit2t1ho",
  "x": "S884v_ljFvHl0Xf6sLbBuxb8vKABeimiFJbEn-pUrkY"
}
"#;
pub static MOCK_JWT_PRIVATE_KEY: LazyLock<Jwk> = LazyLock::new(||
    Jwk::from_bytes(MOCK_JWT_PRIVATE_KEY_STR)
        .expect("failed to parse mock jwt private key")
);

pub const MOCK_JWT_PUBLIC_KEY_STR: &str = r#"{
  "kty": "OKP",
  "use": "sig",
  "crv": "Ed25519",
  "x": "S884v_ljFvHl0Xf6sLbBuxb8vKABeimiFJbEn-pUrkY"
}
"#;
pub static MOCK_JWT_PUBLIC_KEY: LazyLock<Jwk> = LazyLock::new(||
    Jwk::from_bytes(MOCK_JWT_PUBLIC_KEY_STR)
        .expect("failed to parse mock jwt public key")
);

pub const MOCK_PEPPER_STR: &str = "fgnE/aRrTLhILyWy/cICQg==";
pub static MOCK_PEPPER: LazyLock<Box<[u8]>> = LazyLock::new(||
    Base64::decode_vec(MOCK_PEPPER_STR)
        .expect("failed to decode mock pepper")
        .into()
);
