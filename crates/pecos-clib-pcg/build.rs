use cc::Build;
use std::env;
use std::path::PathBuf;

fn main() {
    let clib_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("clibs")
        .join("pecos-rng");

    let src_path = clib_path.join("src");

    println!(
        "cargo:rerun-if-changed={}",
        src_path.join("rng_pcg.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        src_path.join("rng_pcg.h").display()
    );

    Build::new()
        .file(src_path.join("rng_pcg.c"))
        .include(&src_path)
        .compile("pecos_pcg");
}
