use std::{env, ffi::OsString, io, path::PathBuf, process::{Command, Stdio}};

use tap::Tap;

const CSS_NAME: &str = "dumbnotesd-web.css";
const TEMPLATES_PATH: &str = "../dumbnotesd/templates";

// TODO: extract into a module
fn main() -> io::Result<()> {
    println!("cargo::rerun-if-changed=package.json");
    println!("cargo::rerun-if-changed=pnpm-lock.json");
    println!("cargo::rerun-if-changed=pnpm-workspace.json");
    println!("cargo::rerun-if-changed=node_modules");
    println!("cargo::rerun-if-changed={CSS_NAME}");

    assert!(
        PathBuf::from(TEMPLATES_PATH).is_dir(),
        "template directory not found at {TEMPLATES_PATH}",
    );
    println!("cargo::rerun-if-changed={TEMPLATES_PATH}");

    let status = Command::new("pnpm")
        .arg("ci")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()?;
    assert!(status.success(), "failed installing tailwindcss");

    let status = Command::new("pnx")
        .arg("@tailwindcss/cli")
        .arg("--minify")
        .arg(
            OsString::from("--input=")
                .tap_mut(|arg| arg.push(CSS_NAME))
        )
        .arg(
            OsString::from("--output=")
                .tap_mut(|arg|
                    arg.push(
                        PathBuf
                            ::from(
                                env::var_os("OUT_DIR")
                                    .expect("build directory not found")
                            )
                            .tap_mut(|path| path.push(CSS_NAME))
                    )
                )
        )
        .status()?;
    assert!(status.success(), "failed compiling {CSS_NAME}");

    Ok(())
}
