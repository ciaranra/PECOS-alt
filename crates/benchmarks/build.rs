fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // On macOS, explicitly link against the system C++ library
    // This is needed because benchmarks depends on pecos which depends on C++ simulator crates
    // that require libunwind for C++ exception handling at runtime
    if std::env::var("TARGET")
        .unwrap_or_default()
        .contains("darwin")
    {
        println!("cargo:rustc-link-lib=dylib=c++");

        // Add rpath to find system C++ library at runtime
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/lib"
        );
    }
}
