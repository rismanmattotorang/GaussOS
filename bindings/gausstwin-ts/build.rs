use std::process::Command;

fn main() {
    // Only run wasm-pack build when targeting wasm32-unknown-unknown
    if std::env::var("TARGET").unwrap_or_default() == "wasm32-unknown-unknown" {
        // Run wasm-pack build
        let status = Command::new("wasm-pack")
            .args(&["build", "--target", "web"])
            .status()
            .expect("Failed to run wasm-pack build");

        if !status.success() {
            panic!("wasm-pack build failed");
        }
    }

    // Rebuild if any of these files change
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
} 