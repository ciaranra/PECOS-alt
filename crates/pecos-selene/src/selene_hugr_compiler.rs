//! Wrapper around Selene's HUGR to LLVM compiler
//!
//! This module shows how to use Selene's HUGR compiler from pure Rust.
//! Note: This requires adding selene-hugr-qis-compiler as a dependency,
//! which currently has `PyO3` conflicts.

// Example of how to use Selene's HUGR compiler from Rust
// (Currently not functional due to dependency conflicts)

/*
use anyhow::Result;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::OptimizationLevel;

// These would come from selene-hugr-qis-compiler
use selene_hugr_qis_compiler::{
    compile as selene_compile,
    process_hugr,
    get_native_target_machine,
    CompileArgs,
};

/// Compile HUGR to LLVM using Selene's compiler
pub fn compile_hugr_with_selene<'c>(
    context: &'c Context,
    hugr: &mut hugr_core_013::Hugr,  // Note: Uses HUGR 0.13
    name: &str,
    opt_level: OptimizationLevel,
) -> Result<Module<'c>> {
    // Process the HUGR (applies quantum-specific passes)
    process_hugr(hugr)?;

    // Get target machine
    let target_machine = get_native_target_machine(opt_level)?;

    // Set up compilation arguments
    let args = CompileArgs::new(name, &target_machine, opt_level);

    // Compile to LLVM
    let module = selene_compile(&args, context, hugr)?;

    Ok(module)
}
*/

// For now, we document how Selene's compiler works:
//
// 1. Selene uses `tket` which bundles its own version of HUGR with LLVM support
// 2. The compiler is in selene-compilers/hugr_qis/rust/lib.rs
// 3. Key functions:
//    - process_hugr(): Applies quantum-specific optimization passes
//    - compile(): Main compilation function
//    - codegen_extensions(): Sets up quantum operation code generation
//
// The compiler is designed for the Helios QIS and includes:
// - Quantum operation lowering (H, CNOT, measurements, etc.)
// - Future/promise support for deferred measurements
// - Array lowering for quantum registers
// - Debug extensions for quantum state inspection
//
// To use it from Rust, you would need to:
// 1. Resolve the PyO3 dependency conflicts
// 2. Add selene-hugr-qis-compiler as a dependency
// 3. Use the compile() function directly
//
// Alternatively, you could:
// 1. Extract the core compilation logic into a separate crate
// 2. Use it through Python via the PyO3 bindings
// 3. Call the selene Python module from Rust via PyO3

/// Check if Selene HUGR compiler is available
#[must_use]
pub fn is_selene_hugr_compiler_available() -> bool {
    // Currently always false due to dependency conflicts
    false
}
