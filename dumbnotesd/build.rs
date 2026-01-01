use std::io;

include!("protobuf/build.rs");

fn main() -> io::Result<()> {
    build_protobuf(&["protobuf/api_v1.proto"])
}
