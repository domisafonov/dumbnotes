use std::env;
use std::path::PathBuf;
use std::process::Command;
use assert_fs::prelude::*;
use assert_fs::TempDir;

mod common;

#[test]
#[ignore]
fn test() {
    let dir = setup_config();
    // Command::new("env").spawn().unwrap().wait().unwrap();
    let result = Command::new(env::var("CARGO").unwrap())
        .arg("build")
        .arg("--release")
        .arg("--bin=dumbnotes_gen")
        .current_dir(env::var("CARGO_MANIFEST_DIR").unwrap())
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
    assert!(result.success());
    let bin_path = get_bin_path("dumbnotes_gen");
    let result = Command::new(bin_path)
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
    assert!(result.success());

    todo!("chroot and launch")
}

fn setup_config() -> TempDir {
    let root = TempDir::new().unwrap();
    let config_dir = root.child("etc/dumbnotes");
    config_dir.create_dir_all().unwrap();
    let ro_secrets_dir = config_dir.child("private");
    ro_secrets_dir.create_dir_all().unwrap();
    let data_dir = root.child("var/dumbnotes");
    data_dir.create_dir_all().unwrap();
    let rw_secrets_dir = data_dir.child("private");
    rw_secrets_dir.create_dir_all().unwrap();

    config_dir.child("dumbnotes.toml").touch().unwrap();

    root
}

fn get_bin_path(bin_name: &str) -> PathBuf {
    let bin = get_target_dir().join("release").join(bin_name);
    assert!(bin.exists());
    bin
}

fn get_target_dir() -> PathBuf {
    let mut dir = Some(PathBuf::from(env::var("OUT_DIR").unwrap()));
    while let Some(ref d) = dir {
        if d.file_name().unwrap() == "target" {
            return d.to_path_buf();
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    panic!("target dir not found")
}
