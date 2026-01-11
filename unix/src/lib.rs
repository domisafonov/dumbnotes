use std::ffi::CString;
use std::fs::Metadata;
use std::io;
use std::io::ErrorKind;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::Child;
use std::sync::LazyLock;
use libc::{c_int, gid_t, mode_t, uid_t};
use crate::constants::UMASK;
use crate::errors::CheckAccessError;

mod constants;
pub mod errors;

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
        CheckType::File,
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
        CheckType::File,
    )?;
    // TODO: currently unveil prevents the checks
    //  collect paths from the components, validate them, and then unveil
    if cfg!(not(target_os = "openbsd")) {
        check_secret_at_least(
            path.parent().unwrap(),
            7,
            CheckType::Directory,
        )?;
    }
    recursive_check_secret_parent_access(path.parent())
}

fn check_secret_at_least(
    path: &Path,
    required: u32,
    check_type: CheckType,
) -> Result<(), CheckAccessError> {
    let metadata = path.metadata().map_err(map_permissions_error)?;
    match check_type {
        CheckType::File => if !metadata.is_file() {
            return Err(CheckAccessError::NotFile)
        },
        CheckType::Directory => if !metadata.is_dir() {
            return Err(CheckAccessError::NotDirectory)
        },
    }
    let EffectiveMode { our, others } = get_effective_mode(metadata);
    if our & required != required {
        return Err(CheckAccessError::InsufficientPermissions)
    }
    if our != required || others != 0 {
        return Err(
            match check_type {
                CheckType::File => CheckAccessError::FileTooPermissive,
                CheckType::Directory => CheckAccessError::DirectoryHierarchyTooPermissive,
            }
        )
    }
    Ok(())
}
#[derive(Debug)]
enum CheckType {
    File,
    Directory,
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
        return Err(CheckAccessError::DirectoryHierarchyTooPermissive)
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
    } else if metadata.gid() != 0
        && STAFF_GID.map(|gid| gid != metadata.gid()).unwrap_or(true)
    {
        others |= (mode >> 3) & 7
    }
    our |= mode & 7;
    others |= mode & 7;
    EffectiveMode {
        our,
        others,
    }
}

static STAFF_GID: LazyLock<Option<gid_t>> = LazyLock::new(|| {
    if cfg!(target_os = "macos") {
        Some(
            getgrnam_r("staff")
                .expect("getgrnam_r failed")
                .expect("no group \"staff\"")
        )
    } else {
        None
    }
});

fn map_permissions_error(e: io::Error) -> CheckAccessError {
    match e.kind() {
        ErrorKind::PermissionDenied =>
            CheckAccessError::InsufficientPermissions,
        _ => CheckAccessError::Io(e),
    }
}

pub fn is_root() -> bool {
    (unsafe { libc::getuid() }) == 0
}

pub fn set_umask() {
    let default = unsafe { libc::umask(UMASK) };
    unsafe { libc::umask(default | UMASK) };
}

pub fn getpwnam_r(username: &str) -> Result<Option<(uid_t, gid_t)>, io::Error> {
    let username = CString::new(username)?;
    let buf_size = unsafe { libc::sysconf(libc::_SC_GETPW_R_SIZE_MAX) };
    if buf_size == -1 {
        return Err(io::Error::last_os_error())
    }
    let buf_size = buf_size as usize;
    let mut buffer = Box::<[libc::c_char]>::new_uninit_slice(buf_size);
    let mut passwd = MaybeUninit::<libc::passwd>::uninit();
    let mut out_ptr = MaybeUninit::<*mut libc::passwd>::uninit();
    let res = unsafe {
        libc::getpwnam_r(
            username.as_ptr(),
            passwd.as_mut_ptr(),
            buffer.as_mut_ptr().cast(),
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

pub fn getgrnam_r(groupname: &str) -> Result<Option<gid_t>, io::Error> {
    let groupname = CString::new(groupname)?;
    let buf_size = unsafe { libc::sysconf(libc::_SC_GETGR_R_SIZE_MAX) };
    if buf_size == -1 {
        return Err(io::Error::last_os_error())
    }
    let buf_size = buf_size as usize;
    let mut buffer = Box::<[libc::c_char]>::new_uninit_slice(buf_size);
    let mut group = MaybeUninit::<libc::group>::uninit();
    let mut out_ptr = MaybeUninit::<*mut libc::group>::uninit();
    let res = unsafe {
        libc::getgrnam_r(
            groupname.as_ptr(),
            group.as_mut_ptr(),
            buffer.as_mut_ptr().cast(),
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

pub fn chmod(path: &Path, mode: mode_t) -> Result<(), io::Error> {
    let path = CString::new(path.as_os_str().as_bytes())?;
    if unsafe { libc::chmod(path.as_ptr(), mode) } == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub trait ChildKillTermExt {
    fn kill_term(&self) -> Result<(), io::Error>;
}
impl ChildKillTermExt for Child {
    fn kill_term(&self) -> Result<(), io::Error> {
        match unsafe { libc::kill(self.id().cast_signed(), libc::SIGTERM) } {
            -1 => Err(io::Error::last_os_error()),
            _ => Ok(()),
        }
    }
}

pub trait FdNonblockExt {
    fn is_nonblock(&self) -> Result<bool, io::Error>;
    fn set_nonblock(&self, is_nonblock: bool) -> Result<(), io::Error>;
}
impl<T: AsRawFd> FdNonblockExt for T {
    fn is_nonblock(&self) -> Result<bool, io::Error> {
        Ok(
            unsafe { fcntl_raw_int(self.as_raw_fd(), libc::F_GETFL, 0)? }
                & libc::O_NONBLOCK != 0
        )
    }

    fn set_nonblock(&self, is_nonblock: bool) -> Result<(), io::Error> {
        let flags = unsafe {
            fcntl_raw_int(self.as_raw_fd(), libc::F_GETFL, 0)?
        };
        let flags = if is_nonblock {
            flags | libc::O_NONBLOCK
        } else {
            flags & !libc::O_NONBLOCK
        };
        unsafe { fcntl_raw_int(self.as_raw_fd(), libc::F_SETFL, flags)? };
        Ok(())
    }
}

unsafe fn fcntl_raw_int(
    fd: RawFd,
    op: c_int,
    arg1: c_int,
) -> Result<c_int, io::Error> {
    match unsafe { libc::fcntl(fd, op, arg1) } {
        -1 => Err(io::Error::last_os_error()),
        res => Ok(res),
    }
}
