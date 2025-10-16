fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // For macOS, add rpath to find system C++ library at runtime
    // This is needed because pecos-julia-ffi depends on C++ simulator crates
    if std::env::var("TARGET")
        .unwrap_or_default()
        .contains("darwin")
    {
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/lib"
        );
    }
}
