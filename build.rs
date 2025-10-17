use std::io;

fn main() -> io::Result<()> {
    prost_build::compile_protos(&["protobuf/api_v1.proto"], &["protobuf"])
}
