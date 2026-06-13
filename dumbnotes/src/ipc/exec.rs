use std::path::PathBuf;
use thiserror::Error;
use which::which;

#[cfg(target_os = "openbsd")]
const OPENBSD_LIBEXEC_BASE: &str = "/usr/local/libexec/dumbnotesd/";

const AUTH_BIN_NAME: &str = "dumbnotesd-auth";
const STORAGE_BIN_NAME: &str = "dumbnotesd-storage";

#[cfg(target_os = "openbsd")]
pub fn get_authd_executable_path() -> Result<PathBuf, GetExecPathError> {
    get_executable_path(AUTH_BIN_NAME)
}

#[cfg(target_os = "openbsd")]
pub fn get_storaged_executable_path() -> Result<PathBuf, GetExecPathError> {
    get_executable_path(STORAGE_BIN_NAME)
}

#[cfg(target_os = "openbsd")]
fn get_executable_path(bin_name: &str) -> Result<PathBuf, GetExecPathError> {
    if cfg!(all(not(debug_assertions), not(integration_test))) {
        Ok(PathBuf::from(format!("{OPENBSD_LIBEXEC_BASE}/{bin_name}")))
    } else {
        get_executable_path_fallback(bin_name)
    }
}

// TODO: have a configured path for linux too
#[cfg(not(target_os = "openbsd"))]
pub fn get_authd_executable_path() -> Result<PathBuf, GetExecPathError> {
    get_executable_path_fallback(AUTH_BIN_NAME)
}

#[cfg(not(target_os = "openbsd"))]
pub fn get_storaged_executable_path() -> Result<PathBuf, GetExecPathError> {
    get_executable_path_fallback(STORAGE_BIN_NAME)
}

fn get_executable_path_fallback(
    bin_name: &str,
) -> Result<PathBuf, GetExecPathError> {
    let exec_name = std::env::args()
        .next()
        .ok_or(GetExecPathError::NoPathToSelf)?;
    let exec_name = PathBuf::from(exec_name);
    let exec_name = exec_name.parent()
        .ok_or(GetExecPathError::NoSelfParent)?
        .join(bin_name);
    if exec_name.exists() {
        Ok(exec_name.to_owned())
    } else {
        which(bin_name)
            .map_err(GetExecPathError::from)
    }
}

#[derive(Debug, Error)]
pub enum GetExecPathError {
    #[error(transparent)]
    Which(#[from] which::Error),
    
    #[error("no path to self")]
    NoPathToSelf,
    
    #[error("no parent exists for self executable path")]
    NoSelfParent,
}
