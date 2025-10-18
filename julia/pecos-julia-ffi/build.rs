fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // For macOS, link against the system C++ library from dyld shared cache
    if std::env::var("TARGET")
        .unwrap_or_default()
        .contains("darwin")
    {
        println!("cargo:rustc-link-lib=c++");
    }
}
