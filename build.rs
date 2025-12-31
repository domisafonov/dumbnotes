use std::io;

fn main() -> io::Result<()> {
    println!("cargo::rerun-if-changed=protobuf/");
    prost_build::compile_protos(
        &[
            "protobuf/api_v1.proto",
            "protobuf/auth_ipc.proto",
        ],
        &["protobuf"]
    )
}
