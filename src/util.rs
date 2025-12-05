use std::io;
use std::os::fd::AsRawFd;
use libc::{gid_t, uid_t};

pub trait StrExt: AsRef<str> {
    fn nonblank_to_some(&self) -> Option<String> {
        Some(self.as_ref().trim())
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    }
}

impl<T: AsRef<str>> StrExt for T {}

// https://github.com/rust-lang/rust/issues/130113
pub fn send_fut_lifetime_workaround<F: Future + Send>(
    fut: F,
) -> impl Future<Output=F::Output> + Send {
    fut
}

#[macro_export]
macro_rules! error_exit {
    ($($args:tt)*) => ({
        log::error!($($args)*);
        std::process::exit(1)
    });
}

pub fn get_ids() -> (uid_t, gid_t) {
    // SAFETY: a libc call
    //  also assumes that we aren't depending on supplementary group ids
    //  or a separate fs ids
    unsafe { (libc::getuid(), libc::getgid()) }
}

pub trait ChownExt: AsRawFd {
    fn chown(
        &mut self,
        uid: Option<uid_t>,
        gid: Option<gid_t>,
    ) -> io::Result<()> {
        if uid.is_none() && gid.is_none() {
            return Ok(());
        }
        let res = unsafe {
            libc::fchown(
                self.as_raw_fd(),
                uid.unwrap_or(!0),
                gid.unwrap_or(!0),
            )
        };
        if res == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}
impl<T: AsRawFd> ChownExt for T {}
