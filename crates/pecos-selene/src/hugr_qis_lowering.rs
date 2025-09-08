//! HUGR to QIS (Quantum Instruction Set) lowering
//!
//! This module provides a simplified HUGR to LLVM IR compiler that generates
//! QIS-compatible LLVM IR from HUGR 0.13. This is based on Selene's approach
//! but simplified to work without external dependencies.

use anyhow::Result;
use std::collections::HashMap;
use std::fmt::Write;

/// Generate LLVM IR from a quantum circuit description
///
/// This is a simplified version that demonstrates the approach.
/// A full implementation would parse the HUGR and generate appropriate IR.
pub fn generate_quantum_llvm_ir(module_name: &str, entry_point: &str) -> Result<String> {
    let mut llvm_ir = String::new();

    // Module header
    writeln!(&mut llvm_ir, "; ModuleID = '{}'", module_name)?;
    writeln!(&mut llvm_ir, "source_filename = \"{}.hugr\"", module_name)?;
    writeln!(&mut llvm_ir)?;

    // Type declarations
    writeln!(&mut llvm_ir, "; Quantum types")?;
    writeln!(&mut llvm_ir, "%Qubit = type opaque")?;
    writeln!(&mut llvm_ir, "%Result = type opaque")?;
    writeln!(&mut llvm_ir)?;

    // QIS function declarations
    append_qis_declarations(&mut llvm_ir)?;

    // Entry point function
    writeln!(&mut llvm_ir, "; Entry point")?;
    writeln!(&mut llvm_ir, "define void @{}() #0 {{", entry_point)?;
    writeln!(&mut llvm_ir, "entry:")?;

    // Example: Simple quantum circuit
    // In a real implementation, this would be generated from the HUGR
    writeln!(&mut llvm_ir, "  ; Allocate qubits")?;
    writeln!(
        &mut llvm_ir,
        "  %q0 = call %Qubit* @__quantum__qis__qalloc()"
    )?;
    writeln!(
        &mut llvm_ir,
        "  %q1 = call %Qubit* @__quantum__qis__qalloc()"
    )?;
    writeln!(&mut llvm_ir)?;

    writeln!(&mut llvm_ir, "  ; Apply gates")?;
    writeln!(
        &mut llvm_ir,
        "  call void @__quantum__qis__h__body(%Qubit* %q0)"
    )?;
    writeln!(
        &mut llvm_ir,
        "  call void @__quantum__qis__cnot__body(%Qubit* %q0, %Qubit* %q1)"
    )?;
    writeln!(&mut llvm_ir)?;

    writeln!(&mut llvm_ir, "  ; Measure")?;
    writeln!(
        &mut llvm_ir,
        "  %r0 = call %Result* @__quantum__qis__mz__body(%Qubit* %q0)"
    )?;
    writeln!(
        &mut llvm_ir,
        "  %r1 = call %Result* @__quantum__qis__mz__body(%Qubit* %q1)"
    )?;
    writeln!(&mut llvm_ir)?;

    writeln!(&mut llvm_ir, "  ; Free qubits")?;
    writeln!(
        &mut llvm_ir,
        "  call void @__quantum__qis__qfree(%Qubit* %q0)"
    )?;
    writeln!(
        &mut llvm_ir,
        "  call void @__quantum__qis__qfree(%Qubit* %q1)"
    )?;
    writeln!(&mut llvm_ir)?;

    writeln!(&mut llvm_ir, "  ret void")?;
    writeln!(&mut llvm_ir, "}}")?;
    writeln!(&mut llvm_ir)?;

    // Attributes
    writeln!(&mut llvm_ir, "attributes #0 = {{ \"EntryPoint\" }}")?;

    Ok(llvm_ir)
}

/// Append QIS function declarations to the LLVM IR
fn append_qis_declarations(llvm_ir: &mut String) -> Result<()> {
    writeln!(llvm_ir, "; Quantum Instruction Set (QIS) declarations")?;

    // Qubit management
    writeln!(llvm_ir, "declare %Qubit* @__quantum__qis__qalloc()")?;
    writeln!(llvm_ir, "declare void @__quantum__qis__qfree(%Qubit*)")?;

    // Single-qubit gates
    writeln!(llvm_ir, "declare void @__quantum__qis__h__body(%Qubit*)")?;
    writeln!(llvm_ir, "declare void @__quantum__qis__x__body(%Qubit*)")?;
    writeln!(llvm_ir, "declare void @__quantum__qis__y__body(%Qubit*)")?;
    writeln!(llvm_ir, "declare void @__quantum__qis__z__body(%Qubit*)")?;
    writeln!(llvm_ir, "declare void @__quantum__qis__s__body(%Qubit*)")?;
    writeln!(llvm_ir, "declare void @__quantum__qis__t__body(%Qubit*)")?;

    // Rotation gates
    writeln!(
        llvm_ir,
        "declare void @__quantum__qis__rx__body(double, %Qubit*)"
    )?;
    writeln!(
        llvm_ir,
        "declare void @__quantum__qis__ry__body(double, %Qubit*)"
    )?;
    writeln!(
        llvm_ir,
        "declare void @__quantum__qis__rz__body(double, %Qubit*)"
    )?;

    // Two-qubit gates
    writeln!(
        llvm_ir,
        "declare void @__quantum__qis__cnot__body(%Qubit*, %Qubit*)"
    )?;
    writeln!(
        llvm_ir,
        "declare void @__quantum__qis__cz__body(%Qubit*, %Qubit*)"
    )?;

    // Measurement
    writeln!(
        llvm_ir,
        "declare %Result* @__quantum__qis__mz__body(%Qubit*)"
    )?;
    writeln!(
        llvm_ir,
        "declare %Result* @__quantum__qis__mx__body(%Qubit*)"
    )?;
    writeln!(
        llvm_ir,
        "declare %Result* @__quantum__qis__my__body(%Qubit*)"
    )?;
    writeln!(
        llvm_ir,
        "declare void @__quantum__qis__reset__body(%Qubit*)"
    )?;

    // Result operations
    writeln!(
        llvm_ir,
        "declare i1 @__quantum__qis__read_result__body(%Result*)"
    )?;

    writeln!(llvm_ir)?;
    Ok(())
}

/// Map from quantum operation names to QIS function names
pub fn get_qis_op_mapping() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();

    // Basic gates
    map.insert("h", "__quantum__qis__h__body");
    map.insert("x", "__quantum__qis__x__body");
    map.insert("y", "__quantum__qis__y__body");
    map.insert("z", "__quantum__qis__z__body");
    map.insert("s", "__quantum__qis__s__body");
    map.insert("t", "__quantum__qis__t__body");

    // Rotation gates
    map.insert("rx", "__quantum__qis__rx__body");
    map.insert("ry", "__quantum__qis__ry__body");
    map.insert("rz", "__quantum__qis__rz__body");

    // Two-qubit gates
    map.insert("cnot", "__quantum__qis__cnot__body");
    map.insert("cx", "__quantum__qis__cnot__body");
    map.insert("cz", "__quantum__qis__cz__body");

    // Measurement
    map.insert("measure", "__quantum__qis__mz__body");
    map.insert("measurex", "__quantum__qis__mx__body");
    map.insert("measurey", "__quantum__qis__my__body");
    map.insert("reset", "__quantum__qis__reset__body");

    map
}

/// Generate a simple Bell state circuit in LLVM IR
pub fn generate_bell_state_llvm() -> Result<String> {
    let mut llvm_ir = String::new();

    writeln!(&mut llvm_ir, "; Bell state circuit")?;
    writeln!(&mut llvm_ir, "source_filename = \"bell_state.qir\"")?;
    writeln!(&mut llvm_ir)?;

    writeln!(&mut llvm_ir, "%Qubit = type opaque")?;
    writeln!(&mut llvm_ir, "%Result = type opaque")?;
    writeln!(&mut llvm_ir)?;

    append_qis_declarations(&mut llvm_ir)?;

    writeln!(&mut llvm_ir, "define void @bell_state() #0 {{")?;
    writeln!(&mut llvm_ir, "entry:")?;
    writeln!(
        &mut llvm_ir,
        "  %q0 = call %Qubit* @__quantum__qis__qalloc()"
    )?;
    writeln!(
        &mut llvm_ir,
        "  %q1 = call %Qubit* @__quantum__qis__qalloc()"
    )?;
    writeln!(
        &mut llvm_ir,
        "  call void @__quantum__qis__h__body(%Qubit* %q0)"
    )?;
    writeln!(
        &mut llvm_ir,
        "  call void @__quantum__qis__cnot__body(%Qubit* %q0, %Qubit* %q1)"
    )?;
    writeln!(
        &mut llvm_ir,
        "  %r0 = call %Result* @__quantum__qis__mz__body(%Qubit* %q0)"
    )?;
    writeln!(
        &mut llvm_ir,
        "  %r1 = call %Result* @__quantum__qis__mz__body(%Qubit* %q1)"
    )?;
    writeln!(
        &mut llvm_ir,
        "  call void @__quantum__qis__qfree(%Qubit* %q0)"
    )?;
    writeln!(
        &mut llvm_ir,
        "  call void @__quantum__qis__qfree(%Qubit* %q1)"
    )?;
    writeln!(&mut llvm_ir, "  ret void")?;
    writeln!(&mut llvm_ir, "}}")?;
    writeln!(&mut llvm_ir)?;
    writeln!(&mut llvm_ir, "attributes #0 = {{ \"EntryPoint\" }}")?;

    Ok(llvm_ir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qis_op_mapping() {
        let mapping = get_qis_op_mapping();
        assert_eq!(mapping.get("h"), Some(&"__quantum__qis__h__body"));
        assert_eq!(mapping.get("cnot"), Some(&"__quantum__qis__cnot__body"));
        assert_eq!(mapping.get("cx"), Some(&"__quantum__qis__cnot__body"));
    }

    #[test]
    fn test_generate_bell_state() {
        let llvm_ir = generate_bell_state_llvm().unwrap();
        assert!(llvm_ir.contains("@bell_state()"));
        assert!(llvm_ir.contains("__quantum__qis__h__body"));
        assert!(llvm_ir.contains("__quantum__qis__cnot__body"));
    }

    #[test]
    fn test_generate_quantum_llvm_ir() {
        let llvm_ir = generate_quantum_llvm_ir("test_module", "main").unwrap();
        assert!(llvm_ir.contains("ModuleID = 'test_module'"));
        assert!(llvm_ir.contains("@main()"));
        assert!(llvm_ir.contains("%Qubit = type opaque"));
    }
}
