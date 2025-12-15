use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr::null;
use bitflags::{bitflags, bitflags_match};
use libc::{c_char, c_int};
use log::trace;
use crate::error_exit;

pub fn unveil(
    path: &Path,
    permissions: Permissions,
) {
    trace!("unveiling path {}", path.display());
    unsafe { unveil_raw(Some(path), permissions) }
        .unwrap_or_else(|e|
            error_exit!(
                "unable to unveil path \"{}\": {e}",
                path.display()
            )
        );
}

pub fn seal_unveil() {
    trace!("sealing the unveil");
    unsafe { unveil_raw(None, Permissions::empty()) }
        .unwrap_or_else(|e| error_exit!("unable to finalize unveil: {e}"))
}

unsafe fn unveil_raw(
    path: Option<&Path>,
    permissions: Permissions,
) -> Result<(), std::io::Error> {
    unsafe extern "C" {
        pub fn unveil(
            path: *const c_char,
            permissions: *const c_char,
        ) -> c_int;
    }

    let path = path.map(|p|
        CString::new(p.as_os_str().as_bytes()).unwrap()
    );
    let permissions = if permissions.is_empty() {
        None
    } else {
        Some(permissions.into_raw())
    };

    let res = unsafe {
        unveil(
            path.as_ref().map(|s| s.as_ptr()).unwrap_or(null()),
            permissions.as_ref().map(|s| s.as_ptr()).unwrap_or(null()),
        )
    };
    if res == -1 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

bitflags! {
    #[derive(PartialEq)]
    pub struct Permissions: u32 {
        const C = 0b1000;
        const R = 0b100;
        const W = 0b10;
        const X = 0b1;
    }
}

impl Permissions {
    fn into_raw(self) -> CString {
        let mut buf = Vec::with_capacity(
            Permissions::all().bits().count_ones() as usize
        );
        self.iter().for_each(|perm| buf.push(
            bitflags_match!(perm, {
                Permissions::C => b'c',
                Permissions::R => b'r',
                Permissions::W => b'w',
                Permissions::X => b'x',
                _ => error_exit!("invalid flag in permissions: {:#b}", perm),
            })
        ));
        buf.push(0);
        unsafe { CString::from_vec_with_nul_unchecked(buf) }
    }
}
