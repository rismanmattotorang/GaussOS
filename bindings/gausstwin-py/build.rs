fn main() {
    // Enable numpy support
    pyo3_build_config::add_extension_module_link_args();

    // Rebuild if any of these files change
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
} 