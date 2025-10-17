/// This build script helps with `PyO3` configuration.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // For macOS, add required linker args for Python extension modules
    #[cfg(target_os = "macos")]
    {
        pyo3_build_config::add_extension_module_link_args();

        // Link against system C++ library statically to avoid libunwind.1.dylib runtime issues
        // This is needed because pecos-rslib depends on C++ simulator crates
        println!("cargo:rustc-link-lib=static=c++");
        println!("cargo:rustc-link-lib=static=c++abi");
    }
}
