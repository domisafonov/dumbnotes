use argon2::password_hash::PasswordHashString;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct User {
    pub username: String,
    pub hash: PasswordHashString,
}