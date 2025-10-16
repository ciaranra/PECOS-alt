/// This build script helps with `PyO3` configuration.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // For macOS, add required linker args for Python extension modules
    #[cfg(target_os = "macos")]
    {
        pyo3_build_config::add_extension_module_link_args();

        // Add rpath to find system C++ library at runtime
        // This is needed because pecos-rslib depends on C++ simulator crates
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/lib"
        );
    }
}
