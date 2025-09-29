use std::env;

fn main() {
    // Export symbols for dynamic loading
    // This is needed so that plugins loaded with libloading can find our selene_* functions
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "linux" {
        println!("cargo:rustc-link-arg=-Wl,--export-dynamic");
    } else if env::var("CARGO_CFG_TARGET_OS").unwrap() == "macos" {
        println!("cargo:rustc-link-arg=-Wl,-export_dynamic");
    }
}