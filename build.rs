use std::env;
use std::path::PathBuf;

fn main() {
    // Configure protobuf generation - commented out until proto files are available
    /*
    let proto_files = ["proto/agent.proto", "proto/model.proto", "proto/space.proto"];
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .build_client(true)
        .build_server(true)
        .file_descriptor_set_path(out_dir.join("gausstwin_descriptor.bin"))
        .out_dir(&out_dir)
        .compile(&proto_files, &["proto"])
        .unwrap();

    // Rebuild if any of these files change
    for proto_file in &proto_files {
        println!("cargo:rerun-if-changed={}", proto_file);
    }
    */
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Set linker flags for platform-specific optimizations
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "linux" {
        println!("cargo:rustc-link-arg=-Wl,-z,relro,-z,now");
    }
} 