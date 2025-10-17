fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // On macOS, explicitly link against the system C++ library
    // This is needed because benchmarks depends on pecos which depends on C++ simulator crates
    // that require libunwind for C++ exception handling at runtime
    if std::env::var("TARGET")
        .unwrap_or_default()
        .contains("darwin")
    {
        // Link against system C++ library dynamically
        println!("cargo:rustc-link-lib=dylib=c++");

        // Allow undefined symbols to be resolved at runtime
        // This prevents the linker from creating a dependency on libunwind.1.dylib
        // since libunwind is embedded in libc++ on modern macOS
        println!("cargo:rustc-link-arg=-Wl,-undefined,dynamic_lookup");
    }
}
