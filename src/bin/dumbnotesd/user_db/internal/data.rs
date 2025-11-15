use argon2::password_hash::PasswordHashString;
use serde::{Deserialize, Serialize};
use crate::user_db::internal::user::User;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UserData {
    pub username: String,

    #[serde(with = "dumbnotes::serde::password_hash_string")]
    pub hash: PasswordHashString,
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
