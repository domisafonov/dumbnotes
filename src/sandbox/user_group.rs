use std::ffi::CString;
use std::io;
use std::io::ErrorKind;
use std::mem::MaybeUninit;
use libc::{gid_t, uid_t};
use log::debug;

pub fn clear_supplementary_groups() -> Result<(), io::Error> {
    let supplementary_groups: [gid_t; 0] = [0; 0];
    let res = unsafe { libc::setgroups(0, supplementary_groups.as_ptr()) };
    if res == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn set_user_and_group(user_group: &str) -> Result<(), io::Error> {
    debug!("setting user and group to \"{user_group}\"");
    let (uid, gid) = get_user_and_group(user_group)?
        .ok_or_else(||
            io::Error::new(
                ErrorKind::NotFound,
                "user or group \"{user_group}\" not found"
            )
        )?;

    let res = unsafe { libc::setgid(gid) };
    if res == -1 {
        return Err(io::Error::last_os_error());
    }

    let res = unsafe { libc::setuid(uid) };
    if res == -1 {
        return Err(io::Error::last_os_error())
    }

    Ok(())
}

pub fn get_user_and_group(user_group: &str) -> Result<Option<(uid_t, gid_t)>, io::Error> {
    let split: Vec<&str> = user_group.split(':').collect();
    match split.len() {
        1 => getpwnam_r(split[0]),
        2 => {
            let uid = getpwnam_r(split[0])?;
            let gid = getgrnam_r(split[1])?;
            Ok(
                uid.and_then(|(uid, _)|
                    gid.map(|gid| (uid, gid))
                )
            )
        },
        _ => Err(
            io::Error::new(
                ErrorKind::InvalidInput,
                "invalid user and group format: \"{user_group}\"",
            )
        ),
    }
}

fn getpwnam_r(username: &str) -> Result<Option<(uid_t, gid_t)>, io::Error> {
    let username = CString::new(username)?;
    let buf_size = unsafe { libc::sysconf(libc::_SC_GETPW_R_SIZE_MAX) };
    if buf_size == -1 {
        return Err(io::Error::last_os_error())
    }
    let buf_size = buf_size as usize;
    let mut buffer = vec![0; buf_size];
    let mut passwd = MaybeUninit::<libc::passwd>::uninit();
    let mut out_ptr = MaybeUninit::<*mut libc::passwd>::uninit();
    let res = unsafe {
        libc::getpwnam_r(
            username.as_ptr(),
            passwd.as_mut_ptr(),
            buffer.as_mut_ptr(),
            buf_size,
            out_ptr.as_mut_ptr(),
        )
    };
    if res != 0 {
        return Err(io::Error::from_raw_os_error(res))
    }
    Ok(
        if unsafe { out_ptr.assume_init() }.is_null() {
            None
        } else {
            let passwd = unsafe { passwd.assume_init() };
            Some((passwd.pw_uid, passwd.pw_gid))
        }
    )
}

fn getgrnam_r(groupname: &str) -> Result<Option<gid_t>, io::Error> {
    let groupname = CString::new(groupname)?;
    let buf_size = unsafe { libc::sysconf(libc::_SC_GETGR_R_SIZE_MAX) };
    if buf_size == -1 {
        return Err(io::Error::last_os_error())
    }
    let buf_size = buf_size as usize;
    let mut buffer = vec![0; buf_size];
    let mut group = MaybeUninit::<libc::group>::uninit();
    let mut out_ptr = MaybeUninit::<*mut libc::group>::uninit();
    let res = unsafe {
        libc::getgrnam_r(
            groupname.as_ptr(),
            group.as_mut_ptr(),
            buffer.as_mut_ptr(),
            buf_size,
            out_ptr.as_mut_ptr(),
        )
    };
    if res != 0 {
        return Err(io::Error::from_raw_os_error(res))
    }
    Ok(
        if unsafe { out_ptr.assume_init() }.is_null() {
            None
        } else {
            let group = unsafe { group.assume_init() };
            Some(group.gr_gid)
        }
    )
}
