use std::path::Path;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use unix::chmod;
use crate::data::{MOCK_JWT_PRIVATE_KEY_STR, MOCK_JWT_PUBLIC_KEY_STR, MOCK_PEPPER_STR};

pub fn setup_basic_config() -> TempDir {
    setup_basic_config_impl(None::<&str>, None::<&str>)
}

pub fn setup_basic_config_impl(
    data_path: Option<impl AsRef<Path>>,
    user_db_path: Option<impl AsRef<Path>>,
) -> TempDir {
    let root = TempDir::new().unwrap();
    let config_dir = root.child("etc/dumbnotes");
    config_dir.create_dir_all().unwrap();
    let ro_secrets_dir = config_dir.child("private");
    ro_secrets_dir.create_dir_all().unwrap();
    let data_dir = root.child("var/dumbnotes");
    data_dir.create_dir_all().unwrap();
    let rw_secrets_dir = data_dir.child("private");
    rw_secrets_dir.create_dir_all().unwrap();

    let data_path_extra = match data_path {
        Some(path) => format!(
            "data_directory = \"{}\"\n",
            root.child(path).to_str().unwrap(),
        ),
        None => String::new(),
    };
    let user_db_path_extra = match user_db_path {
        Some(path) => format!(
            "user_db = \"{}\"\n",
            root.child(path).to_str().unwrap(),
        ),
        None => String::new(),
    };
    let config = format!(
        r#"jwt_private_key = "{}"
jwt_public_key = "{}"
pepper_path = "{}"
{}{}"#,
        ro_secrets_dir.child("jwt_private_key.json").to_str().unwrap(),
        config_dir.child("jwt_public_key.json").to_str().unwrap(),
        ro_secrets_dir.child("pepper.b64").to_str().unwrap(),
        data_path_extra,
        user_db_path_extra,
    );
    config_dir.child("dumbnotes.toml").write_str(&config).unwrap();

    root
}

pub fn setup_basic_config_with_keys() -> TempDir {
    setup_basic_config_with_keys_impl(None::<&str>, None::<&str>)
}

pub fn setup_basic_config_with_keys_impl(
    data_path: Option<impl AsRef<Path>>,
    user_db_path: Option<impl AsRef<Path>>,
) -> TempDir {
    let root = setup_basic_config_impl(data_path, user_db_path);
    let jwt_private_key = root.child("etc/dumbnotes/private/jwt_private_key.json");
    jwt_private_key.write_str(MOCK_JWT_PRIVATE_KEY_STR).unwrap();
    chmod(jwt_private_key.path(), 0o400).unwrap();
    let pepper_path = root.child("etc/dumbnotes/private/pepper.b64");
    pepper_path.write_str(MOCK_PEPPER_STR).unwrap();
    chmod(pepper_path.path(), 0o400).unwrap();
    root.child("etc/dumbnotes/jwt_public_key.json")
        .write_str(MOCK_JWT_PUBLIC_KEY_STR).unwrap();
    root
}

pub fn setup_basic_config_with_keys_and_data() -> TempDir {
    let user_db_rel_path = "etc/dumbnotes/private/users.toml";
    let data_dir_rel_path = "var/dumbnotes";
    let root = setup_basic_config_with_keys_impl(
        Some(&data_dir_rel_path),
        Some(&user_db_rel_path),
    );
    let user_db = root.child(user_db_rel_path);
    user_db.touch().unwrap();
    chmod(user_db.path(), 0o400).unwrap();
    let data_dir = root.child(data_dir_rel_path);
    data_dir.child("notes")
        .create_dir_all().unwrap();
    let private_data_dir = data_dir.child("private");
    private_data_dir.create_dir_all().unwrap();
    chmod(private_data_dir.path(), 0o700).unwrap();
    let session_db = root.child("var/dumbnotes/private/session.toml");
    session_db.touch().unwrap();
    chmod(session_db.path(), 0o600).unwrap();
    root
}
