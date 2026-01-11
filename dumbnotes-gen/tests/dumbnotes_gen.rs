use std::error::Error;
use std::fs;
use std::io::Write;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use predicates::prelude::*;
use rexpect::reader::Options;
use rexpect::ReadUntil;
use std::process::{Command, Stdio};
use argon2::{Algorithm, Argon2, PasswordHash, PasswordVerifier, Version};
use argon2::password_hash::Encoding;
use base64ct::{Base64, Encoding as Base64Encoding};
use boolean_enums::gen_boolean_enum;
use josekit::jwk::Jwk;
use rexpect::session::PtySession;
use dumbnotes::config::hasher_config::ProductionHasherConfigData;
use test_utils::{new_configured_command, setup_basic_config, setup_basic_config_with_keys, ChildKillOnDropExt, PtySessionExt, GEN_BIN_PATH};
use test_utils::data::MOCK_PEPPER;
use test_utils::predicates::file_mode;

// TODO: ownership, overwriting, default paths (requires chroot)

#[test]
fn create_jwt_key() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config();

    call_create(&dir, "--generate-jwt-key")?;
    let private_key = dir.child("etc/dumbnotes/private/jwt_private_key.json");
    let public_key = dir.child("etc/dumbnotes/jwt_public_key.json");
    private_key
        .assert(
            predicates::path::is_file()
                .and(file_mode(0o400, 0o337))
        );
    public_key
        .assert(
            predicates::path::is_file()
                .and(file_mode(0o440, 0o133))
        );

    let private_key = Jwk::from_bytes(fs::read_to_string(&private_key)?)?;
    let public_key = Jwk::from_bytes(fs::read_to_string(&public_key)?)?;
    assert_ne!(private_key.to_public_key()?, private_key);
    assert_eq!(private_key.to_public_key()?, public_key);

    Ok(())
}

#[test]
fn create_pepper() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config();

    call_create(&dir, "--generate-pepper")?;
    let pepper = dir.child("etc/dumbnotes/private/pepper.b64");
    pepper
        .assert(
            predicates::path::is_file()
                .and(file_mode(0o400, 0o337))
        );

    Base64::decode_vec(fs::read_to_string(&pepper)?.trim())?;

    Ok(())
}

#[test]
fn hash_with_created_secrets() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config();

    call_create(&dir, "--generate-pepper")?;
    let pepper = Base64::decode_vec(
        fs::read_to_string(
            dir.child("etc/dumbnotes/private/pepper.b64")
        )?.trim()
    )?;

    let mut result = new_gen_command(&dir)
        .arg("--no-repeat")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .kill_on_drop();

    result.stdin.take().expect("failed to get stdin")
        .write_all("123".as_bytes())?;
    let result = result.into_child().wait_with_output()?;
    let stdout = String::from_utf8(result.stdout)?;
    let stderr = String::from_utf8(result.stderr)?;
    assert!(result.status.success(), "status: {}", result.status);
    assert!(stderr.is_empty(), "stderr: {stderr}");
    validate_hash_custom_pepper(&pepper, stdout.trim(), "123")?;

    Ok(())
}

#[test]
fn hash_password_empty() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys();

    let mut child = spawn(&dir)?;
    child.exp_string("Enter the password:")?;
    child.send_line("")?;
    child.exp_string("entered password is empty")?;
    child.assert_exit_failure()?;

    Ok(())
}

#[test]
fn hash_password_empty_ctrl_d() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys();

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
    let dir = setup_basic_config_with_keys();

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
    let dir = setup_basic_config_with_keys();

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
    let dir = setup_basic_config_with_keys();

    let mut child = new_gen_command(&dir)
        .arg("--no-repeat")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .kill_on_drop();
    child.stdin.as_mut()
        .expect("failed to get stdin")
        .write_all("123".as_bytes())?;
    let result = child.into_child().wait_with_output()?;
    let output = String::from_utf8(result.stdout)?;
    let errors = String::from_utf8(result.stderr)?;
    assert!(errors.is_empty(), "stderr: {errors}");
    validate_hash(output.trim(), "123")?;

    Ok(())
}

#[test]
fn hash_password_no_repeat_empty() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys();

    let mut child = new_gen_command(&dir)
        .arg("--no-repeat")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .kill_on_drop();
    child.stdin.take();
    let result = child.into_child().wait_with_output()?;
    let output = String::from_utf8(result.stdout)?;
    let err = String::from_utf8(result.stderr)?;
    assert!(output.is_empty(), "stdout: {output}");
    assert!(err.contains("ERROR"), "stderr: {err}");
    assert!(!result.status.success(), "status: {}", result.status);

    Ok(())
}

#[test]
fn hash_password_spaces_warning() -> Result<(), Box<dyn Error>> {
    let dir = setup_basic_config_with_keys();
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
    let mut child = new_gen_command(dir)
        .arg("--no-repeat")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .kill_on_drop();
    child.stdin.take()
        .expect("failed to get stdin")
        .write_all(password.as_bytes())?;
    let result = child.into_child().wait_with_output()?;
    let output = String::from_utf8(result.stdout)?;
    let err = String::from_utf8(result.stderr)?;
    validate_hash(output.trim(), password)?;
    if must_warn.into() {
        assert!(
            err.contains("the password has leading or trailing whitespace characters"),
            "stderr: {err}",
        );
    } else {
        assert!(err.is_empty(), "stderr: {err}");
    }
    assert!(result.status.success(), "status: {}", result.status);
    Ok(())
}
gen_boolean_enum!(MustWarn);

fn call_create(
    dir: &TempDir,
    arg: &str,
) -> Result<(), Box<dyn Error>> {
    let result = new_gen_command(dir)
        .arg(arg)
        .spawn()?
        .wait()?;
    assert!(result.success(), "status: {result}");
    Ok(())
}

fn new_gen_command(dir: &TempDir) -> Command {
    new_configured_command(&GEN_BIN_PATH, dir)
}

fn spawn(
    dir: &TempDir,
) -> Result<PtySession, Box<dyn Error>> {
    let mut child = rexpect::spawn_with_options(
        new_gen_command(dir),
        Options {
            timeout_ms: Some(5000),
            ..Options::default()
        },
    )?;
    child.process.set_kill_timeout(Some(1000));
    Ok(child)
}

fn validate_hash(
    hash: &str,
    password: &str,
) -> Result<(), Box<dyn Error>> {
    validate_hash_custom_pepper(
        &MOCK_PEPPER,
        hash,
        password,
    )
}

fn validate_hash_custom_pepper(
    pepper: &[u8],
    hash: &str,
    password: &str,
) -> Result<(), Box<dyn Error>> {
    let hasher = Argon2::new_with_secret(
        pepper,
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
