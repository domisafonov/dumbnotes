use std::fmt::Formatter;
use base64ct::{Base64, Encoding};
use serde::de::Error;
use serde::{Deserializer, Serializer};
use serde::de::Unexpected::Str;

pub fn serialize<S: Serializer>(
    data: impl AsRef<[u8]>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(
        &Base64::encode_string(data.as_ref())
    )
}

pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<u8>, D::Error> {
    struct Base64Visitor;

    impl<'de> serde::de::Visitor<'de> for Base64Visitor {
        type Value = Vec<u8>;
        fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a base64 encoded string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Base64::decode_vec(v)
                .map_err(|e| Error::invalid_value(Str(v), &self))
        }
    }

    deserializer.deserialize_str(Base64Visitor)
}
