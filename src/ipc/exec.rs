use std::path::PathBuf;
use which::which;

/// Get executable path for authd
///
/// # Safety
/// panics on errors, call only during process startup
pub unsafe fn get_authd_executable_path() -> PathBuf {
    if cfg!(all(target_os = "openbsd", not(debug_assertions))) {
        PathBuf::from("/usr/local/libexec/dumbnotesd/dumbnotesd_auth")
    } else {
        // TODO: have a configured path for linux too

        let exec_name = std::env::args()
            .next().expect("no path to executable");
        let exec_name = PathBuf::from(exec_name);
        let exec_name = exec_name.parent()
            .expect("no parent directory for executable path");
        if exec_name.exists() {
            exec_name.to_owned()
        } else {
            which("dumbnotesd_auth")
                .expect("dumbnotesd_auth not found")
        }
    }
}
