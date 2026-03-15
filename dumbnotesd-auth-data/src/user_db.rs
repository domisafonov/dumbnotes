use argon2::PasswordHash;
use data::User;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UserData {
    pub username: String,

    #[serde(with = "crate::serde::password_hash")]
    pub hash: PasswordHash,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UsersData {
    #[serde(default, rename = "user")]
    pub users: Vec<UserData>,
}

impl From<UserData> for User {
    fn from(value: UserData) -> Self {
        User {
            username: value.username,
            hash: value.hash,
        }
    }
}
