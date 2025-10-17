fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // For macOS, link against system C++ library to avoid libunwind.1.dylib dependency
    // Use static linking to avoid runtime dependency issues
    if std::env::var("TARGET")
        .unwrap_or_default()
        .contains("darwin")
    {
        // Link against system C++ library statically to avoid runtime dependencies
        println!("cargo:rustc-link-lib=static=c++");
        println!("cargo:rustc-link-lib=static=c++abi");
    }
}
