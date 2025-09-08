use std::env;
use std::path::PathBuf;

fn main() {
    // Get the target directory
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = PathBuf::from(out_dir);
    let target_dir = out_path
        .ancestors()
        .find(|p| p.ends_with("target"))
        .expect("Could not find target directory")
        .to_path_buf();

    // Determine the profile (debug or release)
    let profile = env::var("PROFILE").unwrap();

    // Set the path to the bytesim plugin
    let plugin_name = if cfg!(target_os = "windows") {
        "pecos_selene_plugins.dll"
    } else if cfg!(target_os = "macos") {
        "libpecos_selene_plugins.dylib"
    } else {
        "libpecos_selene_plugins.so"
    };

    let plugin_path = target_dir.join(profile).join(plugin_name);

    // Export the path as an environment variable for the crate
    println!(
        "cargo:rustc-env=PECOS_BYTESIM_PLUGIN_PATH={}",
        plugin_path.display()
    );

    // Ensure the plugin is rebuilt if it changes
    println!("cargo:rerun-if-changed=../pecos-selene-plugins/src/lib.rs");
}
