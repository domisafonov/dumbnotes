use rocket::serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct User {
    pub username: String,
    pub hash: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UsersData {
    #[serde(rename = "user")]
    pub users: Vec<User>,
}
