use std::io;

include!("protobuf/build.rs");

fn main() -> io::Result<()> {
    build_protobuf(&["protobuf/auth_ipc.proto"])
}
