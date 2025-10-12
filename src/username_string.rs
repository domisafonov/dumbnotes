use std::fmt::Formatter;
use std::ops::Deref;
use std::str::FromStr;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Error;
use serde::de::Unexpected::Str;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UsernameString(String);

impl FromStr for UsernameString {
    type Err = UsernameParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(UsernameString(s.to_string())) // TODO: the validation
    }
}

impl Deref for UsernameString {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0[..]
    }
}

#[derive(Debug)]
pub struct UsernameParseError;

impl Serialize for UsernameString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for UsernameString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UsernameString;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("string containing a valid username")
            }

            fn visit_str<E>(self, v: &str) -> Result<UsernameString, E>
            where
                E: Error
            {
                UsernameString::from_str(v)
                    .map_err(|e| Error::invalid_value(Str(v), &self))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}
