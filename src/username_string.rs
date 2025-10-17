use std::borrow::Borrow;
use std::fmt::Formatter;
use std::ops::Deref;
use std::str::FromStr;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Error;
use serde::de::Unexpected::Str;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UsernameString(String);

#[derive(Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct UsernameStr(str);

impl UsernameString {
    fn as_str(&self) -> &str {
        &self.0
    }
}

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

impl Deref for UsernameStr {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl Borrow<UsernameStr> for UsernameString {
    fn borrow(&self) -> &UsernameStr {
        // SAFETY: relies on UsernameStr being repr(transparent),
        // holding a single string slice
        unsafe { std::mem::transmute(&self.0[..]) }
    }
}

impl ToOwned for UsernameStr {
    type Owned = UsernameString;

    fn to_owned(&self) -> Self::Owned {
        UsernameString(self.0.to_owned())
    }
}

#[derive(Debug)]
pub struct UsernameParseError;

impl Serialize for UsernameStr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl Serialize for UsernameString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_str().serialize(serializer)
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

            fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("string containing a valid username")
            }

            fn visit_str<E>(self, v: &str) -> Result<UsernameString, E>
            where
                E: Error
            {
                UsernameString::from_str(v)
                    .map_err(|_| Error::invalid_value(Str(v), &self))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}
