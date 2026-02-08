use std::fmt::Formatter;
use argon2::password_hash::{Encoding, PasswordHashString};
use argon2::PasswordHash;
use serde::{Deserializer, Serializer};
use serde::de::Error;
use serde::de::Unexpected::Str;

pub fn serialize<S: Serializer>(
    data: &PasswordHashString,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(
        data.as_str()
    )
}

pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<PasswordHashString, D::Error> {
    struct PhcVisitor;

    impl<'de> serde::de::Visitor<'de> for PhcVisitor {
        type Value = PasswordHashString;

        fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a base64 encoded string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            PasswordHash::parse(v, Encoding::B64)
                .map(|h| h.serialize())
                .map_err(|_| Error::invalid_value(Str(v), &self))
        }
    }

    deserializer.deserialize_str(PhcVisitor)
}
