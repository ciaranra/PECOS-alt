/// This build script helps with `PyO3` configuration.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // For macOS, add required linker args for Python extension modules
    #[cfg(target_os = "macos")]
    {
        pyo3_build_config::add_extension_module_link_args();

        // Link against system C++ library dynamically
        // This is needed because pecos-rslib depends on C++ simulator crates
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
