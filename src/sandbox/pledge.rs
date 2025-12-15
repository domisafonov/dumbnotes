use std::ffi::{c_char, c_int, CString};
use std::ptr::null;
use log::trace;
use crate::error_exit;

pub fn pledge_init() {
    pledge(
        Some("stdio rpath wpath cpath tmppath inet fattr unix getpw proc exec id unveil"),

        // copied from authd
        Some("stdio rpath wpath cpath flock unix getpw unveil"),
    )
}

// TODO: does it make sense to repledge after the rocket starts serving?
// TODO: repledge after self-forking too
pub fn pledge_liftoff() {
    trace!("pledging for continuous operation");
    pledge(
        Some("stdio rpath wpath cpath tmppath inet fattr"),
        None,
    )
}

// unix is for initializing syslog
pub fn pledge_authd_init() {
    pledge(
        Some("stdio rpath wpath cpath flock unix getpw unveil"),
        None,
    )
}

pub fn pledge_authd_normal() {
    trace!("pledging for continuous operation");
    pledge(
        Some("stdio rpath wpath cpath flock"),
        None,
    )
}

pub fn pledge_gen_init() {
    pledge(
        Some("stdio rpath wpath cpath tty getpw chown fattr unveil"),
        None,
    )
}

pub fn pledge_gen_key() {
    trace!("pledging for generating the keys");
    pledge(
        Some("stdio rpath wpath cpath getpw chown fattr"),
        None,
    )
}

pub fn pledge_gen_hash() {
    trace!("pledging for generating a password hash");
    pledge(
        Some("stdio rpath tty"),
        None,
    )
}

fn pledge(
    promises: Option<&str>,
    execpromises: Option<&str>,
) {
    unsafe { pledge_raw(promises, execpromises) }
        .unwrap_or_else(|e| error_exit!("unable to pledge: {e}"));
}

unsafe fn pledge_raw(
    promises: Option<&str>,
    execpromises: Option<&str>,
) -> Result<(), std::io::Error> {
    unsafe extern "C" {
        pub fn pledge(
            promises: *const c_char,
            execpromises: *const c_char,
        ) -> c_int;
    }

    let promises = promises.map(|s| CString::new(s).unwrap());
    let execpromises = execpromises.map(|s| CString::new(s).unwrap());
    let res = unsafe {
        pledge(
            promises.as_ref().map(|s| s.as_ptr()).unwrap_or(null()),
            execpromises.as_ref().map(|s| s.as_ptr()).unwrap_or(null()),
        )
    };
    if res == -1 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}
