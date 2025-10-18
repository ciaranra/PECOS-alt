/// This build script helps with `PyO3` configuration.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // For macOS, add required linker args for Python extension modules
    #[cfg(target_os = "macos")]
    {
        pyo3_build_config::add_extension_module_link_args();

        // Link against the system C++ library from dyld shared cache
        // Prioritize /usr/lib to prevent opportunistic linking to Homebrew's libunwind
        println!("cargo:rustc-link-search=native=/usr/lib");
        println!("cargo:rustc-link-lib=c++");
        println!("cargo:rustc-link-arg=-Wl,-search_paths_first");
    }
}
