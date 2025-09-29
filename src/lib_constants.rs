pub const DEFAULT_USER_DB: &str = "/etc/dumbnotes/users";
pub const DEFAULT_DATA_DIR: &str = "/var/dumbnotes";

// TODO: validate to fit both in u64 and usize
// TODO: use static-assertions crate for the defaults?
pub const DEFAULT_MAX_NOTE_LEN: u64 = 128 * 1024;
pub const DEFAULT_MAX_NOTE_NAME_LEN: u64 = 256;
