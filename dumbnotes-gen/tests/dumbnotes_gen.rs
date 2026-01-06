use std::error::Error;
use std::io::Write;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use predicates::prelude::*;
use rexpect::process::wait::WaitStatus;
use rexpect::reader::Options;
use rexpect::ReadUntil;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::LazyLock;
use argon2::{Algorithm, Argon2, PasswordHash, PasswordVerifier, Version};
use argon2::password_hash::Encoding;
use boolean_enums::gen_boolean_enum;
use rexpect::session::PtySession;
use dumbnotes::config::hasher_config::ProductionHasherConfigData;
use test_utils::build_bin;
use test_utils::data::{MOCK_JWT_PRIVATE_KEY_STR, MOCK_JWT_PUBLIC_KEY_STR, MOCK_PEPPER, MOCK_PEPPER_STR};
use test_utils::predicates::file_mode;

// TODO: generated files' format
// TODO: ownership, overwriting, default paths (requires chroot)

static GEN_BIN: LazyLock<PathBuf> = LazyLock::new(|| {
    build_bin("dumbnotes-gen")
        .unwrap_or_else(|e| panic!("build failed: {e}"))
});

#[test]
fn create_jwt_key() -> Result<(), Box<dyn Error>> {
    let dir = setup_config();
    call_create(&dir, "--generate-jwt-key")?;
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
    Ok(())
}

#[test]
fn create_pepper() -> Result<(), Box<dyn Error>> {
    let dir = setup_config();
    call_create(&dir, "--generate-pepper")?;
    dir.child("etc/dumbnotes/private/pepper.b64")
        .assert(
            predicates::path::is_file()
                .and(file_mode(0o400, 0o337))
        );
    Ok(())
}

#[test]
fn hash_password_empty() -> Result<(), Box<dyn Error>> {
    let dir = setup_config_with_keys();

    let mut child = spawn(&dir)?;
    child.exp_string("Enter the password:")?;
    child.send_line("")?;
    child.exp_string("entered password is empty")?;
    child.assert_exit_failure()?;

    Ok(())
}

#[test]
fn hash_password_empty_ctrl_d() -> Result<(), Box<dyn Error>> {
    let dir = setup_config_with_keys();

    let mut child = spawn(&dir)?;
    child.exp_string("Enter the password:")?;
    child.send_control('d')?;
    child.exp_regex(".*ERROR.*")?;
    child.assert_exit_failure()?;

    let mut child = spawn(&dir)?;
    child.exp_string("Enter the password:")?;
    child.send_line("123")?;
    child.exp_string("Repeat the password:")?;
    child.send_control('d')?;
    child.exp_regex(".*ERROR.*")?;
    child.assert_exit_failure()?;

    Ok(())
}

#[test]
fn hash_password_unmatched() -> Result<(), Box<dyn Error>> {
    let dir = setup_config_with_keys();

    let mut child = spawn(&dir)?;
    child.exp_string("Enter the password:")?;
    child.send_line("123")?;
    child.exp_string("Repeat the password:")?;
    child.send_line("456")?;
    child.exp_string("the passwords do not match")?;
    child.assert_exit_failure()?;

    Ok(())
}

#[test]
fn hash_password() -> Result<(), Box<dyn Error>> {
    let dir = setup_config_with_keys();

    let mut child = spawn(&dir)?;
    child.exp_string("Enter the password:")?;
    child.send_line("123")?;
    child.exp_string("Repeat the password:")?;
    child.send_line("123")?;
    let (_, output) = child.reader.read_until(&ReadUntil::EOF)?;
    let output = output.trim();
    validate_hash(output.trim(), "123")?;
    child.assert_exit_success()?;

    Ok(())
}

#[test]
fn hash_password_no_repeat() -> Result<(), Box<dyn Error>> {
    let dir = setup_config_with_keys();

    let mut child = new_command(&dir)
        .arg("--no-repeat")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child.stdin.as_mut()
        .expect("failed to get stdin")
        .write_all("123".as_bytes())?;
    let result = child.wait_with_output()?;
    let output = String::from_utf8(result.stdout)?;
    let errors = String::from_utf8(result.stderr)?;
    assert!(errors.is_empty());
    validate_hash(output.trim(), "123")?;

    Ok(())
}

#[test]
fn hash_password_no_repeat_empty() -> Result<(), Box<dyn Error>> {
    let dir = setup_config_with_keys();

    let mut child = new_command(&dir)
        .arg("--no-repeat")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child.stdin.take();
    let result = child.wait_with_output()?;
    let output = String::from_utf8(result.stdout)?;
    let err = String::from_utf8(result.stderr)?;
    assert!(output.is_empty());
    assert!(err.contains("ERROR"));
    assert!(!result.status.success());

    Ok(())
}

#[test]
fn hash_password_spaces_warning() -> Result<(), Box<dyn Error>> {
    let dir = setup_config_with_keys();
    hash_password_spaces_impl(&dir, " 123", MustWarn::Yes)?;
    hash_password_spaces_impl(&dir, "1 \t23", MustWarn::No)?;
    hash_password_spaces_impl(&dir, "123 ", MustWarn::Yes)?;
    Ok(())
}

fn hash_password_spaces_impl(
    dir: &TempDir,
    password: &str,
    must_warn: MustWarn,
) -> Result<(), Box<dyn Error>> {
    let mut child = new_command(dir)
        .arg("--no-repeat")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child.stdin.take().unwrap().write_all(password.as_bytes())?;
    let result = child.wait_with_output()?;
    let output = String::from_utf8(result.stdout)?;
    let err = String::from_utf8(result.stderr)?;
    validate_hash(output.trim(), password)?;
    if must_warn.into() {
        assert!(err.contains("the password has leading or trailing whitespace characters"));
    } else {
        assert!(err.is_empty());
    }
    assert!(result.status.success());
    Ok(())
}
gen_boolean_enum!(MustWarn);

fn call_create(
    dir: &TempDir,
    arg: &str,
) -> Result<(), Box<dyn Error>> {
    let result = new_command(dir)
        .arg(arg)
        .spawn()?
        .wait()?;
    assert!(result.success());
    Ok(())
}

fn new_command(dir: &TempDir) -> Command {
    let mut command = Command::new(&*GEN_BIN);
    command
        .arg(
            format!(
                "--config-file={}",
                dir.join("etc/dumbnotes/dumbnotes.toml")
                    .to_str().expect("failed to get config path")
            )
        );
    command
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

fn setup_config_with_keys() -> TempDir {
    let root = setup_config();
    root.child("etc/dumbnotes/private/jwt_private_key.json")
        .write_str(MOCK_JWT_PRIVATE_KEY_STR).unwrap();
    root.child("etc/dumbnotes/private/pepper.b64")
        .write_str(MOCK_PEPPER_STR)
        .unwrap();
    root.child("etc/dumbnotes/jwt_public_key.json")
        .write_str(MOCK_JWT_PUBLIC_KEY_STR).unwrap();
    root
}

fn spawn(
    dir: &TempDir,
) -> Result<PtySession, Box<dyn Error>> {
    let mut child = rexpect::spawn_with_options(
        new_command(dir),
        Options {
            timeout_ms: Some(1000),
            ..Options::default()
        },
    )?;
    child.process.set_kill_timeout(Some(1000));
    Ok(child)
}

trait PtySessionExt {
    fn assert_exit_success(&mut self) -> Result<(), Box<dyn Error>> {
        assert_eq!(self.get_exit_code()?, 0);
        Ok(())
    }

    fn assert_exit_failure(&mut self) -> Result<(), Box<dyn Error>> {
        assert_ne!(self.get_exit_code()?, 0);
        Ok(())
    }

    fn get_exit_code(&mut self) -> Result<i32, Box<dyn Error>>;
}
impl PtySessionExt for PtySession {
    fn get_exit_code(&mut self) -> Result<i32, Box<dyn Error>> {
        self.exp_eof()?;
        let result = self.process.wait()?;
        match result {
            WaitStatus::Exited(_, exit_code) => Ok(exit_code),
            _ => panic!("failed to get exit code"),
        }
    }
}

fn validate_hash(hash: &str, password: &str) -> Result<(), Box<dyn Error>> {
    let hasher = Argon2::new_with_secret(
        &MOCK_PEPPER,
        Algorithm::Argon2id,
        Version::V0x13,
        ProductionHasherConfigData::default().make_params()?,
    )?;
    hasher
        .verify_password(
            password.as_bytes(),
            &PasswordHash::parse(hash, Encoding::B64)?
        )?;
    Ok(())
}
