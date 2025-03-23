pub struct UsernameString(String);

// TODO: extract to the settings struct
// TODO: validate to fit both in u64 and usize
// TODO: use static-assertions crate for the defaults?
pub const MAX_NOTE_LEN: u64 = 128 * 1024;
pub const MAX_NOTE_NAME_LEN: u64 = 256;

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
