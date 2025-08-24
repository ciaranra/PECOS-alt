// Build script to compile FFI stub library automatically

use std::process::Command;
use std::path::Path;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    
    // Source file path
    let stub_source = Path::new(&manifest_dir).join("selene_correct_stubs.c");
    let stub_output = Path::new(&out_dir).join("libselene_correct_stubs.so");
    
    // Check if stub source exists, if not copy from parent directory
    if !stub_source.exists() {
        let parent_stub = Path::new(&manifest_dir).parent().unwrap().parent().unwrap().join("selene_correct_stubs.c");
        if parent_stub.exists() {
            std::fs::copy(&parent_stub, &stub_source).expect("Failed to copy stub source");
        } else {
            panic!("Could not find selene_correct_stubs.c source file");
        }
    }
    
    println!("cargo:rerun-if-changed={}", stub_source.display());
    
    // Compile the stub library
    let output = Command::new("gcc")
        .args(&[
            "-shared", 
            "-fPIC", 
            "-o", stub_output.to_str().unwrap(),
            stub_source.to_str().unwrap()
        ])
        .output()
        .expect("Failed to execute gcc");
    
    if !output.status.success() {
        panic!(
            "Failed to compile stub library:\nstdout: {}\nstderr: {}", 
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    
    println!("cargo:rustc-env=SELENE_STUB_LIB_PATH={}", stub_output.display());
    
    // Also copy to target directory for easy access
    let target_dir = Path::new(&out_dir).parent().unwrap().parent().unwrap().parent().unwrap();
    let target_stub = target_dir.join("libselene_correct_stubs.so");
    let _ = std::fs::copy(&stub_output, &target_stub);
    
    println!("Compiled FFI stub library: {}", stub_output.display());
}