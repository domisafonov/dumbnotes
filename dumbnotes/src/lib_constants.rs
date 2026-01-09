use libc::mode_t;

// TODO: validate to fit both in u64 and usize
// TODO: use static-assertions crate for the defaults?
// TODO: validate the limits to match each other on startup
pub const DEFAULT_MAX_NOTE_LEN: u64 = 128 * 1024;
pub const DEFAULT_MAX_NOTE_NAME_LEN: u64 = 256;

// the defaults are taken from the argon2 crate itself
// TODO: check that the defaults are sane
pub const DEFAULT_ARGON2_M_COST: u32 = 19 * 1024;
pub const DEFAULT_ARGON2_T_COST: u32 = 2;
pub const DEFAULT_ARGON2_P_COST: u32 = 1;
pub const DEFAULT_ARGON2_OUTPUT_LEN: Option<usize> = Some(32);

// relative to the data directory
pub const NOTES_DIRECTORY_PATH: &str = "notes";
