fn build_protobuf(protos: &[impl AsRef<std::path::Path>]) -> std::io::Result<()> {
    for proto in protos {
        println!("cargo::rerun-if-changed={}", proto.as_ref().to_str().unwrap());
    }
    prost_build::compile_protos(
        protos,
        &["protobuf"]
    )
}
