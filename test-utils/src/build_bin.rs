use std::clone::Clone;
use std::env;
use std::env::JoinPathsError;
use std::ffi::OsString;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::LazyLock;
use assert_fs::TempDir;
use cargo_metadata::{CrateType, Message};
use thiserror::Error;

pub static GEN_BIN_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    build_bin(&["dumbnotes-gen"])
        .map(|v| v.into_iter().next().unwrap())
        .unwrap_or_else(|e| panic!("build failed: {e}"))
});

pub static DAEMON_BIN_PATHS: LazyLock<Vec<PathBuf>> = LazyLock::new(||
    build_bin(&["dumbnotesd", "dumbnotesd-auth"])
        .expect("failed to build dumbnotesd")
);
pub static DAEMON_BIN_PATH: LazyLock<PathBuf> = LazyLock::new(||
    DAEMON_BIN_PATHS[0].clone()
);
pub static AUTHD_BIN_PATH: LazyLock<PathBuf> = LazyLock::new(||
    DAEMON_BIN_PATHS[1].clone()
);

pub fn build_bin(names: &[&str]) -> Result<Vec<PathBuf>, BuildBinError> {
    let build_output = call_build(names)?;
    let (ok, err): (Vec<_>, Vec<_>) = names.iter()
        .map(|name| get_build_path(name, &build_output))
        .partition(Result::is_ok);
    match err.into_iter().next() {
        Some(err) => Err(err.unwrap_err()),
        None => Ok(ok.into_iter().map(Result::unwrap).collect())
    }
}

fn call_build(names: &[&str]) -> Result<Vec<Message>, BuildBinError> {
    let manifest_dir = AsRef::<Path>
    ::as_ref(&env::var("CARGO_MANIFEST_DIR")?)
        .parent().expect("no parent for CARGO_MANIFEST_DIR")
        .to_owned();
    let mut command = Command::new(env::var("CARGO")?);
    command
        .arg("build")
        .arg("--profile=integration-test")
        .arg("--config").arg(r#"build.rustflags = ["--cfg=integration_test"]"#)
        .arg("--message-format=json")
        .stdout(Stdio::piped())
        .current_dir(manifest_dir);
    for name in names {
        command.arg(format!("--bin={name}"));
    }
    let mut child = command.spawn()?;

    let build_output = Message
    ::parse_stream(
        BufReader::new(
            child.stdout.take()
                .ok_or(BuildBinError::MissingStdout)?
        )
    )
        .collect::<Result<Vec<Message>, _>>();

    let status = child.wait()?;
    if !status.success() {
        return Err(BuildBinError::ChildUnsuccessful(status))
    }

    build_output.map_err(BuildBinError::from)
}

fn get_build_path(
    name: &str,
    build_output: &[Message],
) -> Result<PathBuf, BuildBinError> {
    build_output.iter()
        .find_map(|message| {
            if let Message::CompilerArtifact(message) = message
                && message.target.crate_types.contains(&CrateType::Bin)
                && message.target.name == name
                && let Some(ref executable) = message.executable
            {
                Some(PathBuf::from(executable))
            } else {
                None
            }
        })
        .ok_or(BuildBinError::NoBinFound)
}

pub fn make_path_for_bins(
    paths: &[impl AsRef<Path>],
) -> Result<OsString, BuildBinError> {
    let mut process_path: Vec<_> = env::split_paths(&env::var("PATH")?)
        .collect();
    for path in paths {
        let Some(path) = path.as_ref().parent() else {
            continue
        };
        if process_path.iter().any(|v| *v == path) {
            continue
        }
        process_path.insert(0, path.to_owned());
    }
    Ok(env::join_paths(&process_path)?)
}

#[derive(Debug, Error)]
pub enum BuildBinError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Var(#[from] env::VarError),

    #[error("missing stdout for child process")]
    MissingStdout,

    #[error("child process failed: {0}")]
    ChildUnsuccessful(ExitStatus),

    #[error("no executable found")]
    NoBinFound,

    #[error("failed to form PATH for child processes: {0}")]
    JoinPaths(#[from] JoinPathsError),
}

pub fn new_configured_command(
    bin_path: &Path,
    dir: &TempDir,
) -> Command {
    new_configured_command_with_env(
        bin_path,
        dir,
        None::<&[PathBuf]>,
    )
}

pub fn new_configured_command_with_env(
    bin_path: &Path,
    dir: &TempDir,
    env_paths: Option<&[impl AsRef<Path>]>,
) -> Command {
    let mut command = Command::new(bin_path);
    command
        .arg(
            format!(
                "--config-file={}",
                dir.join("etc/dumbnotes/dumbnotes.toml")
                    .to_str().expect("failed to get config path")
            )
        );
    if let Some(env_paths) = env_paths {
        command
            .env(
                "PATH",
                make_path_for_bins(env_paths)
                    .unwrap_or_else(|e|
                        panic!("failed to assemble PATH: {e}")
                    )
            );
    }
    command
}
