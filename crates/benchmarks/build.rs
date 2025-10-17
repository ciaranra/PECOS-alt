fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // On macOS, don't explicitly link C++ library
    // Let the system handle it implicitly to avoid libunwind issues
    // The C++ code will still link, but without creating hard dependencies
    if std::env::var("TARGET")
        .unwrap_or_default()
        .contains("darwin")
    {
        // No explicit C++ linking needed - system handles it implicitly
    }
}
