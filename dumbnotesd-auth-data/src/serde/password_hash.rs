use std::fmt::Formatter;
use std::str::FromStr;
use argon2::PasswordHash;
use serde::{Deserializer, Serializer};
use serde::de::Error;
use serde::de::Unexpected::Str;

pub fn serialize<S: Serializer>(
    data: &PasswordHash,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(
        &data.to_string()
    )
}

pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<PasswordHash, D::Error> {
    struct PhcVisitor;

    impl<'de> serde::de::Visitor<'de> for PhcVisitor {
        type Value = PasswordHash;

        fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a base64 encoded string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            PasswordHash::from_str(v)
                .map_err(|_| Error::invalid_value(Str(v), &self))
        }
    }

    deserializer.deserialize_str(PhcVisitor)
}
