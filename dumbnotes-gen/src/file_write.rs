use std::fs::{OpenOptions, Permissions};
use std::io;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;
use libc::{gid_t, uid_t};
use dumbnotes::sandbox::user_group::get_user_and_group;
use unix::{get_ids, ChownExt};

pub fn get_ids_for_chown(
    owner_user_group: Option<&str>,
) -> Result<(Option<uid_t>, Option<gid_t>), io::Error> {
    let owner_user_group = match owner_user_group {
        Some(value) => value,
        None => return Ok((None, None)),
    };
    let (expected_uid, expected_gid) = get_user_and_group(owner_user_group)?;
    let (uid, gid) = get_ids();
    Ok((
        Some(expected_uid).filter(|e| *e != uid),
        Some(expected_gid).filter(|e| *e != gid),
    ))
}

pub fn write(
    path: &Path,
    contents: impl AsRef<str>,
    uid: Option<uid_t>,
    gid: Option<gid_t>,
    mode: Option<u32>,
) -> Result<(), io::Error> {
    let mut options = OpenOptions::new();
    if let Some(mode) = mode {
        options.mode(mode);
    }
    let mut file = options
        .read(false)
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    let metadata = file.metadata()?;
    let (written_uid, written_gid) = (metadata.uid(), metadata.gid());
    if let Some(mode) = mode {
        file.set_permissions(Permissions::from_mode(mode))?;
    }
    file.chown(
        uid.filter(|uid| *uid != written_uid),
        gid.filter(|gid| *gid != written_gid),
    )?;
    file.write_all(contents.as_ref().as_bytes())?;
    Ok(())
}
