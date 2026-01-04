use assert_fs::prelude::*;
use assert_fs::TempDir;
use predicates::prelude::*;
use rexpect::process::wait::WaitStatus;
use rexpect::reader::Options;
use rexpect::ReadUntil;
use std::path::Path;
use std::process::Command;
use test_utils::build_bin;
use test_utils::predicates::file_mode;

// TODO: format, warnings
// TODO: ownership, overwriting, default paths (requires chroot)

#[test]
fn create_jwt_key() {
    let dir = setup_config();
    let bin_path = build_bin("dumbnotes-gen")
        .expect("build failed");
    call(&dir, &bin_path, "--generate-jwt-key");
    dir.child("etc/dumbnotes/private/jwt_private_key.json")
        .assert(
            predicates::path::is_file()
                .and(file_mode(0o400, 0o337))
        );
    dir.child("etc/dumbnotes/jwt_public_key.json")
        .assert(
            predicates::path::is_file()
                .and(file_mode(0o440, 0o133))
        );
}

#[test]
fn create_pepper() {
    let dir = setup_config();
    let bin_path = build_bin("dumbnotes-gen")
        .expect("build failed");
    call(&dir, &bin_path, "--generate-pepper");
    dir.child("etc/dumbnotes/private/pepper.b64")
        .assert(
            predicates::path::is_file()
                .and(file_mode(0o400, 0o337))
        );
}

// TODO: no-repeat, unmatched passwords, empty password
#[test]
fn hash_password() -> Result<(), Box<dyn std::error::Error>> {
    let dir = setup_config();
    let bin_path = build_bin("dumbnotes-gen")
        .expect("build failed");

    // TODO: ship ready-made files
    call(&dir, &bin_path, "--generate-jwt-key");
    call(&dir, &bin_path, "--generate-pepper");

    let mut command = Command::new(bin_path);
    command
        .arg(
            format!(
                "--config-file={}",
                dir.join("etc/dumbnotes/dumbnotes.toml")
                    .to_str().expect("failed to get config path")
            )
        );
    let mut child = rexpect::spawn_with_options(
        command,
        Options {
            timeout_ms: Some(1000),
            ..Options::default()
        },
    )?;
    child.exp_string("Enter the password:")?;
    child.send_line("123")?;
    child.exp_string("Repeat the password:")?;
    child.send_line("123")?;
    let (_, output) = child.reader.read_until(&ReadUntil::EOF)?;
    let output = output.trim();
    assert!(output.starts_with("$argon2id$")); // TODO: parse
    let result = child.process.wait()?;
    let exit_code = match result {
        WaitStatus::Exited(_, exit_code) => Some(exit_code),
        _ => None,
    };
    assert_eq!(Some(0), exit_code);
    Ok(())
}

fn call(dir: &TempDir, bin_path: &Path, arg: &str) {
    let result = Command::new(bin_path)
        .arg(
            format!(
                "--config-file={}",
                dir.join("etc/dumbnotes/dumbnotes.toml")
                    .to_str().expect("failed to get config path")
            )
        )
        .arg(arg)
        .spawn()
        .expect("failed to execute process")
        .wait()
        .unwrap();
    assert!(result.success());
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
