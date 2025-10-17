/// This build script helps with `PyO3` configuration.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // For macOS, add required linker args for Python extension modules
    #[cfg(target_os = "macos")]
    {
        pyo3_build_config::add_extension_module_link_args();

        // Don't explicitly link C++ library - let the system handle it implicitly
        // This avoids creating hard dependencies on libunwind.1.dylib
        // The C++ code from pecos-quest will still link correctly
    }
}
