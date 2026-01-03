use std::env;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use cargo_metadata::{CrateType, Message};
use thiserror::Error;

pub fn build_bin(name: &str) -> Result<PathBuf, BuildBinError> {
    let build_output = call_build(name)?;
    get_build_path(name, &build_output)
}

fn call_build(name: &str) -> Result<Vec<Message>, BuildBinError> {
    let mut child = Command
    ::new(env::var("CARGO")?)
        .arg("build")
        .arg("--release")
        .arg(format!("--bin={name}"))
        .arg("--message-format=json")
        .stdout(Stdio::piped())
        .current_dir(env::var("CARGO_MANIFEST_DIR")?)
        .spawn()?;

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
}