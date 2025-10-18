/// This build script helps with `PyO3` configuration.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // For macOS, add required linker args for Python extension modules
    #[cfg(target_os = "macos")]
    {
        pyo3_build_config::add_extension_module_link_args();

        // Link against the system C++ library from dyld shared cache
        // The -lc++ directive without dylib= prefix allows the linker to use the system library
        println!("cargo:rustc-link-lib=c++");
    }
}
