use std::fmt::Formatter;
use argon2::password_hash::Encoding;
use argon2::PasswordHash;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Error;
use serde::de::Unexpected::Str;
use serde::de::Visitor;
use crate::user_db::internal::user::User;

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UserData<'a> {
    pub username: String,

    #[serde(borrow)]
    pub hash: PasswordHashWrapper<'a>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UsersData<'a> {
    #[serde(borrow, rename = "user")]
    pub users: Vec<UserData<'a>>,
}

#[derive(Clone, Eq, PartialEq)]
pub(super) struct PasswordHashWrapper<'a>(pub PasswordHash<'a>);

impl<'de: 'a, 'a> Deserialize<'de> for PasswordHashWrapper<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PasswordHashWrapperVisitor;
        impl<'de> Visitor<'de> for PasswordHashWrapperVisitor {
            type Value = PasswordHashWrapper<'de>;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a PHC hash string")
            }

            fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                PasswordHash::parse(v, Encoding::B64)
                    .map(PasswordHashWrapper)
                    .map_err(|_| Error::invalid_value(Str(v), &self))
            }
        }

        deserializer.deserialize_str(PasswordHashWrapperVisitor)
    }
}

impl<'a> Serialize for PasswordHashWrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.serialize().as_str())
    }
}

impl<'a> From<UserData<'a>> for User {
    fn from(value: UserData<'a>) -> Self {
        User {
            username: value.username,
            hash: value.hash.0.into(),
        }
    }
}
