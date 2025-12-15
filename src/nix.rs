use std::fs::Metadata;
use std::io;
use std::io::ErrorKind;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use libc::{gid_t, uid_t};
use thiserror::Error;
use crate::lib_constants::UMASK;

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

// TODO: symlinks?
pub fn check_secret_file_ro_access(
    path: &Path,
) -> Result<(), CheckAccessError> {
    if !path.is_absolute() {
        return Err(CheckAccessError::PathNotAbsolute)
    }
    check_secret_at_least(
        path,
        4,
        |m| if m.is_file() {
            Ok(())
        } else {
            Err(CheckAccessError::NotFile)
        }
    )?;
    recursive_check_secret_parent_access(path.parent())
}

// TODO: symlinks?
pub fn check_secret_file_rw_access(
    path: &Path,
) -> Result<(), CheckAccessError> {
    if !path.is_absolute() {
        return Err(CheckAccessError::PathNotAbsolute)
    }
    check_secret_at_least(
        path,
        6,
        |m| if m.is_file() {
            Ok(())
        } else {
            Err(CheckAccessError::NotFile)
        }
    )?;
    // TODO: currently unveil prevents the checks
    //  collect paths from the components, validate them, and then unveil
    if cfg!(not(target_os = "openbsd")) {
        check_secret_at_least(
            path.parent().unwrap(),
            7,
            |m| if m.is_dir() {
                Ok(())
            } else {
                Err(CheckAccessError::NotDirectory)
            }
        )?;
    }
    recursive_check_secret_parent_access(path.parent())
}

fn check_secret_at_least(
    path: &Path,
    required: u32,
    type_checker: impl FnOnce(&Metadata) -> Result<(), CheckAccessError>,
) -> Result<(), CheckAccessError> {
    let metadata = path.metadata().map_err(map_permissions_error)?;
    type_checker(&metadata)?;
    let EffectiveMode { our, others } = get_effective_mode(metadata);
    if our & required != required {
        return Err(CheckAccessError::InsufficientPermissions)
    }
    if our != required || others != 0 {
        return Err(CheckAccessError::TooPermissive)
    }
    Ok(())
}

fn recursive_check_secret_parent_access(
    path: Option<&Path>,
) -> Result<(), CheckAccessError> {
    // TODO: currently unveil prevents the checks
    //  collect paths from the components, validate them, and then unveil
    if cfg!(target_os = "openbsd") {
        return Ok(())
    }

    let path = match path {
        Some(path) => path,
        None => return Ok(()),
    };

    let EffectiveMode { others, .. } = get_effective_mode(
        path.metadata().map_err(map_permissions_error)?,
    );
    if others & 2 != 0 {
        return Err(CheckAccessError::TooPermissive)
    }

    recursive_check_secret_parent_access(path.parent())
}

pub fn check_dir_rw_access(
    path: &Path,
) -> Result<(), CheckAccessError> {
    if !path.is_absolute() {
        return Err(CheckAccessError::PathNotAbsolute)
    }
    let metadata = metadata_or_not_found(path)?;
    if !metadata.is_dir() {
        return Err(CheckAccessError::NotDirectory)
    }
    if get_effective_mode(metadata).our == 7 {
        Ok(())
    } else {
        Err(CheckAccessError::InsufficientPermissions)
    }
}

fn metadata_or_not_found(
    path: &Path,
) -> Result<Metadata, CheckAccessError> {
    let result = path.metadata();
    if matches!(result, Err(ref e) if e.kind() == ErrorKind::NotFound) {
        return Err(CheckAccessError::NotFound)
    }
    result.map_err(map_permissions_error)
}

struct EffectiveMode {
    our: u32,
    others: u32,
}

fn get_effective_mode(metadata: Metadata) -> EffectiveMode {
    let mode = metadata.mode();
    let (uid, gid) = get_ids();
    let mut our = 0;
    let mut others = 0;
    if uid == 0 {
        our |= 7;
    }
    if metadata.uid() == uid {
        our |= (mode >> 6) & 7
    } else if metadata.uid() != 0 {
        others |= (mode >> 6) & 7
    }
    if metadata.gid() == gid {
        our |= (mode >> 3) & 7
    } else if metadata.gid() != 0 {
        others |= (mode >> 3) & 7
    }
    our |= mode & 7;
    others |= mode & 7;
    EffectiveMode {
        our,
        others,
    }
}

fn map_permissions_error(e: io::Error) -> CheckAccessError {
    match e.kind() {
        ErrorKind::PermissionDenied =>
            CheckAccessError::InsufficientPermissions,
        _ => CheckAccessError::Io(e),
    }
}

#[derive(Debug, Error)]
pub enum CheckAccessError {
    #[error(transparent)]
    Io(io::Error),

    #[error("not a directory")]
    NotDirectory,

    #[error("not a file")]
    NotFile,

    #[error("insufficient permissions")]
    InsufficientPermissions,

    #[error("too permissive")]
    TooPermissive,

    #[error("not an absolute path")]
    PathNotAbsolute,

    #[error("not found")]
    NotFound,
}

pub fn is_root() -> bool {
    (unsafe { libc::getuid() }) == 0
}

pub fn set_umask() {
    let default = unsafe { libc::umask(UMASK) };
    unsafe { libc::umask(default | UMASK) };
}
