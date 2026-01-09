use std::io;
use std::io::ErrorKind;
use libc::{gid_t, uid_t};
use log::debug;
use unix::{getgrnam_r, getpwnam_r};

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
    let (uid, gid) = get_user_and_group(user_group)?;

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

pub fn get_user_and_group(user_group: &str) -> Result<(uid_t, gid_t), io::Error> {
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
        .transpose()
        .ok_or_else(||
            io::Error::new(
                ErrorKind::NotFound,
                format!("no user or group \"{user_group}\" found")
            )
        )?
}
