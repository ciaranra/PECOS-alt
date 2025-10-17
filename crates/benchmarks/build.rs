fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // On macOS, explicitly link against the system C++ library
    // Use static linking to avoid libunwind.1.dylib runtime dependency issues
    if std::env::var("TARGET")
        .unwrap_or_default()
        .contains("darwin")
    {
        // Link against system C++ library statically to avoid runtime dependencies
        println!("cargo:rustc-link-lib=static=c++");
        println!("cargo:rustc-link-lib=static=c++abi");
    }
}
