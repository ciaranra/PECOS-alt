use cc::Build;
use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let c_src_dir = manifest_dir.join("c_src");

    // Build our local C code with thread-safe functions exposed
    println!(
        "cargo:rerun-if-changed={}",
        c_src_dir.join("rng_pcg.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        c_src_dir.join("rng_pcg.h").display()
    );

    Build::new()
        .file(c_src_dir.join("rng_pcg.c"))
        .include(&c_src_dir)
        .compile("pecos_pcg");
}
