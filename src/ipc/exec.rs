use std::path::PathBuf;
use thiserror::Error;
use which::which;

pub fn get_authd_executable_path() -> Result<PathBuf, GetExecPathError> {
    if cfg!(all(target_os = "openbsd", not(debug_assertions))) {
        Ok(PathBuf::from("/usr/local/libexec/dumbnotesd/dumbnotesd_auth"))
    } else {
        // TODO: have a configured path for linux too

        let exec_name = std::env::args()
            .next()
            .ok_or(GetExecPathError::NoPathToSelf)?;
        let exec_name = PathBuf::from(exec_name);
        let exec_name = exec_name.parent()
            .ok_or(GetExecPathError::NoSelfParent)?;
        if exec_name.exists() {
            Ok(exec_name.to_owned())
        } else {
            which("dumbnotesd_auth")
                .map_err(GetExecPathError::from)
        }
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