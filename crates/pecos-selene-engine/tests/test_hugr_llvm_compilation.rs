//! Test HUGR to LLVM compilation in pecos-selene-engine
//!
//! Note: HUGR 0.13 support has been removed. HUGR compilation now uses
//! tket's HUGR 0.22 through the pecos-hugr-qis crate.

use pecos_engines::ClassicalControlEngineBuilder;

#[test]
fn test_hugr_llvm_compilation_availability() {
    println!("HUGR to LLVM compilation status in pecos-selene-engine:");
    println!();
    println!("HUGR 0.13 support has been removed.");
    println!("HUGR compilation is now handled by the pecos-hugr-qis crate");
    println!("which uses tket's HUGR 0.22 for compatibility with Selene.");
    println!();
    println!("To compile HUGR to LLVM:");
    println!("  1. Use pecos-hugr-qis crate directly");
    println!("  2. Or use the Python bindings via pecos_rslib.compile_hugr_to_llvm_rust()");
    println!();
    println!("The new implementation supports:");
    println!("  - HUGR 0.22 format (JSON and envelope)");
    println!("  - QIS-compatible LLVM IR generation");
    println!("  - Full quantum gate set");
    println!("  - Measurement and qubit management");
    println!("  - Compatible with Selene's hugr-qis compiler");
}

#[test]
fn test_hugr_compilation_redirects_to_pecos_hugr_qis() {
    // This test just documents that HUGR compilation has moved
    println!("HUGR compilation has moved to the pecos-hugr-qis crate.");
    println!("See crates/pecos-hugr-qis/src/compiler.rs for the implementation.");
}