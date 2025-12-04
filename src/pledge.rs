#![cfg(target_os = "openbsd")]

use std::ffi::{c_char, c_int, CString};
use std::ptr::null;
use log::trace;
use crate::error_exit;

unsafe fn pledge(
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
        // SAFETY: raw_os_error always returns some on last_os_error result
        //  by the docs
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

// unix and ps is for initializing syslog
pub fn pledge_authd_init() {
    unsafe {
        pledge(
            Some("stdio rpath wpath cpath flock unix ps"),
            None,
        )
    }.unwrap_or_else(|e| error_exit!("unable to pledge: {e}"));
}

pub fn pledge_authd_normal() {
    trace!("pledging for continuous operation");
    unsafe {
        pledge(
            Some("stdio rpath wpath cpath flock"),
            None,
        )
    }.unwrap_or_else(|e| error_exit!("unable to pledge: {e}"));
}

pub fn pledge_init() {
    // TODO
}

pub fn pledge_normal() {
    // TODO
}

pub fn pledge_gen_init() {
    // TODO
}

pub fn pledge_gen_normal() {
    // TODO
}
