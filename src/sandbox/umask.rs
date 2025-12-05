use crate::lib_constants::UMASK;

pub fn set_umask() {
    let default = unsafe { libc::umask(UMASK) };
    unsafe { libc::umask(default | UMASK) };
}
