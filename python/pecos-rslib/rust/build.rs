use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=PYO3_PYTHON");
    println!("cargo:rerun-if-env-changed=PYO3_NO_PYTHON");
    println!("cargo:rerun-if-changed=build.rs");

    // Check if we're in test mode
    let is_test = env::var("CARGO_CFG_TEST").is_ok();

    if is_test {
        // During tests, make sure PyO3 doesn't try to find Python
        println!("cargo:rustc-env=PYO3_NO_PYTHON=1");
    }

    // For Windows builds, we need to set PYO3_NO_PYTHON
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-env=PYO3_NO_PYTHON=1");
    }

    // For macOS, ensure proper linking of Python
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-arg=-undefined");
        println!("cargo:rustc-link-arg=dynamic_lookup");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,/Library/Developer/CommandLineTools/Library/Frameworks"
        );
    }
}
