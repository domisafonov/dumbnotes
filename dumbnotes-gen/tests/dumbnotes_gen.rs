use std::env;
use std::io::Read;
use std::process::{Command, Stdio};
use assert_fs::prelude::*;
use assert_fs::TempDir;
use serde_json::Value;

// TODO: the actual tests
#[test]
fn test_poc() {
    let dir = setup_config();
    Command::new("env")
        .spawn().expect("failed to execute env")
        .wait().expect("failed to wait on env child");
    let mut child = Command
        ::new(
            env::var("CARGO").expect("no CARGO variable")
        )
        .arg("build")
        .arg("--release")
        .arg("--bin=dumbnotes-gen")
        .arg("--message-format=json")
        .stdout(Stdio::piped())
        .current_dir(
            env::var("CARGO_MANIFEST_DIR").expect("no CARGO_MANIFEST_DIR variable")
        )
        .spawn()
        .expect("failed to start cargo build");
    let mut build_output = String::new();
    child.stdout
        .as_mut().expect("failed to get the build process's stdout")
        .read_to_string(&mut build_output).expect("failed to read build output");
    assert!(child.wait().expect("waiting for the build process failed").success());
    let bin_path = serde_json::Deserializer::from_str(&build_output)
        .into_iter::<Value>()
        .filter_map(|value| value.ok())
        .filter(|value| value.is_object())
        .filter(|value| value.get("reason") == Some(&Value::from("compiler-artifact")))
        .filter(|value| {
            let target = value.get("target").expect("failed to get target");
            let crate_types = target
                .get("crate_types").expect("failed to get crate_types")
                .as_array().expect("crate types is not an array");
            let name = target.get("name").expect("failed to get name");
            crate_types.contains(&Value::from("bin")) && name == &Value::from("dumbnotes-gen")
        })
        .map(|value|
            value.get("executable").expect("failed to get executable")
                .as_str().expect("failed to get executable")
                .to_owned()
        )
        .next()
        .expect("failed to interpret cargo's output");
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
