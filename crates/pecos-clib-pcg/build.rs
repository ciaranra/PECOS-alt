use cc::Build;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// TODO: Should probably just vendor the C code into the Rust crate...
fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // Try local path first (for development)
    let local_clib_path = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("clib")
        .join("pecos-rng");

    let src_path = if local_clib_path.exists() {
        // Development: use local files
        let src = local_clib_path.join("src");
        println!("cargo:rerun-if-changed={}", src.join("rng_pcg.c").display());
        println!("cargo:rerun-if-changed={}", src.join("rng_pcg.h").display());
        src
    } else {
        // Published crate: download from GitHub
        let pcg_dir = out_dir.join("pcg");
        fs::create_dir_all(&pcg_dir).unwrap();

        let commit = "95a6ddbdf85ad7bcf8b9133aa2552f3f1ae7da84";
        let base_url = format!(
            "https://raw.githubusercontent.com/PECOS-packages/PECOS/{commit}/clib/pecos-rng/src"
        );

        // Download files if they don't exist
        download_if_needed(&pcg_dir.join("rng_pcg.c"), &format!("{base_url}/rng_pcg.c"));
        download_if_needed(&pcg_dir.join("rng_pcg.h"), &format!("{base_url}/rng_pcg.h"));

        pcg_dir
    };

    Build::new()
        .file(src_path.join("rng_pcg.c"))
        .include(&src_path)
        .compile("pecos_pcg");
}

fn download_if_needed(path: &Path, url: &str) {
    if !path.exists() {
        println!("cargo:warning=Downloading {} to {}", url, path.display());

        let mut response = ureq::get(url)
            .call()
            .unwrap_or_else(|e| panic!("Failed to download {url}: {e}"));

        let content = response.body_mut().read_to_vec()
            .unwrap_or_else(|e| panic!("Failed to read response body: {e}"));

        fs::write(path, content)
            .unwrap_or_else(|e| panic!("Failed to write {}: {}", path.display(), e));
    }
}
