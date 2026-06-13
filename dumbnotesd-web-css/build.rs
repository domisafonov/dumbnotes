use std::{env, io, path::PathBuf};

use tap::Tap;

const CSS_NAME: &str = "dumbnotesd-web";

#[cfg(not(target_os = "openbsd"))]
const TEMPLATES_PATH: &str = "../dumbnotesd/templates";

#[cfg(not(target_os = "openbsd"))]
fn main() -> io::Result<()> {
    use std::ffi::OsString;
    use std::process::{Command, Stdio};

    let filename = CSS_NAME.to_string() + ".css";

    println!("cargo::rerun-if-changed=package.json");
    println!("cargo::rerun-if-changed=pnpm-lock.yaml");
    println!("cargo::rerun-if-changed=pnpm-workspace.yaml");
    println!("cargo::rerun-if-changed={filename}");

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
        .tap_mut(|c|
            if env::var_os("CARGO_CFG_DEBUG_ASSERTIONS").is_none() {
                c.arg("--minify");
            }
        )
        .arg(
            OsString::from("--input=")
                .tap_mut(|arg| arg.push(&filename))
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
                            .tap_mut(|path| path.push(&filename))
                    )
                )
        )
        .status()?;
    assert!(status.success(), "failed compiling {filename}");

    Ok(())
}

#[cfg(target_os = "openbsd")]
fn main() -> io::Result<()> {
    use std::fs;

    let variant = if env::var_os("CARGO_CFG_DEBUG_ASSERTIONS").is_some() {
        ".debug"
    } else {
        ".release"
    };
    let filename = CSS_NAME.to_string()
        + variant
        + ".css";

    println!("cargo::rerun-if-changed={filename}");

    fs::copy(
        &filename,
        PathBuf
            ::from(
                env::var_os("OUT_DIR")
                    .expect("build directory not found")
            )
            .tap_mut(|path| path.push(CSS_NAME.to_string() + ".css"))
    ).expect("failed copying built css into the OUT_DIR");

    Ok(())
}
