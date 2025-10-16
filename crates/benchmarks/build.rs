fn main() {
    // On macOS, explicitly link against the system C++ library
    // This is needed because benchmarks depends on pecos which depends on C++ simulator crates
    // that require libunwind for C++ exception handling at runtime
    if std::env::var("TARGET")
        .unwrap_or_default()
        .contains("darwin")
    {
        println!("cargo:rustc-link-lib=dylib=c++");
    }
}
