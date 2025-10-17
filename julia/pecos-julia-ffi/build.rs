fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // For macOS, link against system C++ library to avoid libunwind.1.dylib dependency
    // This is needed because pecos-julia-ffi depends on C++ simulator crates
    if std::env::var("TARGET")
        .unwrap_or_default()
        .contains("darwin")
    {
        // Link against system C++ library dynamically
        println!("cargo:rustc-link-lib=dylib=c++");

        // Add rpath to find system C++ library at runtime
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/lib"
        );

        // Allow undefined symbols to be resolved at runtime
        // This prevents the linker from creating a dependency on libunwind.1.dylib
        // since libunwind is embedded in libc++ on modern macOS
        println!("cargo:rustc-link-arg=-Wl,-undefined,dynamic_lookup");
    }
}
