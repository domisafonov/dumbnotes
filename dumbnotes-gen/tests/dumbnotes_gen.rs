use std::process::Command;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use test_utils::build_bin;

// TODO: the actual tests
#[test]
fn test_poc() {
    let dir = setup_config();
    let bin_path = build_bin("dumbnotes-gen")
        .unwrap();
    let result = Command::new(&bin_path)
        .arg(
            format!(
                "--config-file={}",
                dir.join("etc/dumbnotes/dumbnotes.toml")
                    .to_str().expect("failed to get config path")
            )
        )
        .arg("--generate-jwt-key")
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
    assert!(result.success());
}

// TODO: chrooting
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

    let config = format!(
        r#"jwt_private_key = "{}"
jwt_public_key = "{}"
pepper_path = "{}"
"#,
        ro_secrets_dir.child("jwt_private_key.json").to_str().unwrap(),
        config_dir.child("jwt_public_key.json").to_str().unwrap(),
        ro_secrets_dir.child("pepper.b64").to_str().unwrap(),
    );
    config_dir.child("dumbnotes.toml").write_str(&config).unwrap();

    root
}
