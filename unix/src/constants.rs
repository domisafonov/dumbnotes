use libc::mode_t;

pub const UMASK: mode_t = 0o027;
pub const CHROOT_DIR: &str = "/var/empty";
