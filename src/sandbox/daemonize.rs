use std::ffi::CString;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

/// Daemonize
///
/// # Safety
/// call before creating any async runtimes or other threads
pub unsafe fn daemonize(no_fork: bool) {
    if !no_fork {
        unsafe { fork() };
        setsid();
    }
    let nfd = open_null();
    std::env::set_current_dir("/").expect("cannot change working directory");
    unsafe { replace_fd(&nfd, libc::STDIN_FILENO) };
    unsafe { replace_fd(&nfd, libc::STDOUT_FILENO) };
    unsafe { replace_fd(&nfd, libc::STDERR_FILENO) };
}

unsafe fn fork() {
    let res = unsafe { libc::fork() };
    if res == -1 {
        panic!("fork failed: {}", io::Error::last_os_error());
    }
    if res != 0 {
        std::process::exit(0)
    }
}

fn setsid() {
    let res = unsafe { libc::setsid() };
    if res == -1 {
        panic!("setsid failed: {}", io::Error::last_os_error());
    }
}

fn open_null() -> OwnedFd {
    let dev_null_path = CString::new("/dev/null").unwrap();
    let res = unsafe { libc::open(dev_null_path.as_ptr(), libc::O_RDWR ) };
    if res == -1 {
        panic!("opening /dev/null failed: {}", io::Error::last_os_error());
    }
    unsafe { OwnedFd::from_raw_fd(res) }
}

unsafe fn replace_fd(nfd: &impl AsRawFd, fd: RawFd) {
    let res = unsafe { libc::dup2(nfd.as_raw_fd(), fd) };
    if res == -1 {
        panic!("dup2 failed: {}", io::Error::last_os_error());
    }
}
