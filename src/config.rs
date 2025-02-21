pub struct UsernameString(String);

// TODO: extract to the settings struct
pub const MAX_NOTE_LEN: u64 = 128 * 1024;

#[derive(Debug)]
pub struct UsernameParseError;

impl std::str::FromStr for UsernameString {
    type Err = UsernameParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(UsernameString(s.to_string())) // TODO: the validation
    }
}

impl std::ops::Deref for UsernameString {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0[..]
    }
}
