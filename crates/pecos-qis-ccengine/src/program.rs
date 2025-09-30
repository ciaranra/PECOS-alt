//! Program abstraction for QIS Classical Control Engine
//!
//! This module provides a unified program interface that allows different
//! program types (QisProgram, HUGR, raw QisInterface) to be used with
//! the QisControlEngine through a consistent `.program()` API.

use pecos_core::errors::PecosError;
use pecos_programs::{QisProgram, HugrProgram};
use pecos_qis_interface::{QisInterface, QuantumOp, Operation};
use std::collections::HashMap;

/// A trait for types that can be converted into a QisInterface
///
/// This allows the QisControlEngine builder to accept different program types
/// through a unified `.program()` method, similar to how QASMEngine works.
pub trait IntoQisInterface {
    /// Convert this program into a QisInterface
    ///
    /// # Errors
    /// Returns an error if the conversion fails (e.g., compilation errors,
    /// invalid program format, missing dependencies)
    fn into_qis_interface(self) -> Result<QisInterface, PecosError>;
}

/// Implement IntoQisInterface for QisInterface itself (identity conversion)
impl IntoQisInterface for QisInterface {
    fn into_qis_interface(self) -> Result<QisInterface, PecosError> {
        Ok(self)
    }
}

/// Implement IntoQisInterface for QisProgram
///
/// This compiles/links the LLVM IR into a QisInterface that can be executed
/// by the runtime.
impl IntoQisInterface for QisProgram {
    fn into_qis_interface(self) -> Result<QisInterface, PecosError> {
        match &self.content {
            pecos_programs::QisContent::Ir(ir_text) => {
                qis_ir_to_interface(ir_text)
            }
            pecos_programs::QisContent::Bitcode(_) => {
                Err(PecosError::Generic(
                    "QisProgram bitcode parsing not yet implemented. \
                     Use IR text format instead.".to_string()
                ))
            }
        }
    }
}

/// Implement IntoQisInterface for HUGR bytes
///
/// This compiles HUGR to QIS LLVM IR, then links it into a QisInterface.
impl IntoQisInterface for &[u8] {
    fn into_qis_interface(self) -> Result<QisInterface, PecosError> {
        // Compile HUGR bytes to QIS LLVM IR
        let llvm_ir = pecos_hugr_qis::compile_hugr_bytes_to_string(self)?;

        // Convert to QisProgram and then to QisInterface
        let qis_program = QisProgram::from_string(llvm_ir);
        qis_program.into_qis_interface()
    }
}

/// Implement IntoQisInterface for HUGR bytes (owned)
impl IntoQisInterface for Vec<u8> {
    fn into_qis_interface(self) -> Result<QisInterface, PecosError> {
        self.as_slice().into_qis_interface()
    }
}

/// Implement IntoQisInterface for HugrProgram
///
/// This compiles the HUGR to QIS LLVM IR, then links it into a QisInterface.
impl IntoQisInterface for HugrProgram {
    fn into_qis_interface(self) -> Result<QisInterface, PecosError> {
        // Use the bytes conversion which handles HUGR compilation
        self.into_bytes().into_qis_interface()
    }
}

/// Wrapper type to represent a QIS Control Engine Program
///
/// This is conceptually equivalent to QisInterface, but provides a
/// more semantically clear type name for the builder API.
#[derive(Debug, Clone)]
pub struct QisControlEngineProgram {
    interface: QisInterface,
}

impl QisControlEngineProgram {
    /// Create a new program from a QisInterface
    pub fn new(interface: QisInterface) -> Self {
        Self { interface }
    }

    /// Create a program from anything that can be converted to QisInterface
    ///
    /// # Errors
    /// Returns an error if the conversion fails
    pub fn from_program<P: IntoQisInterface>(program: P) -> Result<Self, PecosError> {
        let interface = program.into_qis_interface()?;
        Ok(Self::new(interface))
    }

    /// Get the underlying QisInterface
    pub fn into_interface(self) -> QisInterface {
        self.interface
    }

    /// Get a reference to the underlying QisInterface
    pub fn interface(&self) -> &QisInterface {
        &self.interface
    }
}

impl IntoQisInterface for QisControlEngineProgram {
    fn into_qis_interface(self) -> Result<QisInterface, PecosError> {
        Ok(self.interface)
    }
}

impl From<QisInterface> for QisControlEngineProgram {
    fn from(interface: QisInterface) -> Self {
        Self::new(interface)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qis_interface_identity_conversion() {
        let interface = QisInterface::new();
        let result = interface.clone().into_qis_interface().unwrap();
        // Basic check that conversion preserves structure
        assert_eq!(result.allocated_qubits, interface.allocated_qubits);
    }

    #[test]
    fn test_qis_control_engine_program_wrapper() {
        let interface = QisInterface::new();
        let program = QisControlEngineProgram::new(interface.clone());

        let back = program.into_interface();
        assert_eq!(back.allocated_qubits, interface.allocated_qubits);
    }

    #[test]
    fn test_qis_program_conversion_basic() {
        // Test with a simple Bell state QIS program
        let bell_llvm = r#"
            define void @main() {
                call void @__quantum__qis__h__body(i64 0)
                call void @__quantum__qis__cx__body(i64 0, i64 1)
                %result0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
                %result1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
                ret void
            }

            declare void @__quantum__qis__h__body(i64)
            declare void @__quantum__qis__cx__body(i64, i64)
            declare i32 @__quantum__qis__m__body(i64, i64)
        "#;

        let qis_program = QisProgram::from_string(bell_llvm);
        let result = qis_program.into_qis_interface();
        assert!(result.is_ok(), "Conversion should succeed: {:?}", result);

        let interface = result.unwrap();
        assert_eq!(interface.allocated_qubits.len(), 2, "Should have 2 qubits");
        assert_eq!(interface.allocated_results.len(), 2, "Should have 2 result slots");
        assert_eq!(interface.operations.len(), 4, "Should have 4 operations");
    }

    #[test]
    fn test_qis_program_conversion_empty() {
        let qis_program = QisProgram::from_string("define void @main() { ret void }");
        let result = qis_program.into_qis_interface();
        assert!(result.is_ok());

        let interface = result.unwrap();
        assert_eq!(interface.allocated_qubits.len(), 0);
        assert_eq!(interface.operations.len(), 0);
    }
}

/// Parse QIS LLVM IR and convert to QisInterface
///
/// This function parses LLVM IR text and extracts QIS function calls to build
/// a QisInterface that can be executed by the QIS runtime.
fn qis_ir_to_interface(ir_text: &str) -> Result<QisInterface, PecosError> {
    let mut interface = QisInterface::new();
    let mut qubit_map = HashMap::new(); // LLVM qubit ID -> interface qubit ID
    let mut result_map = HashMap::new(); // LLVM result ID -> interface result ID

    // Parse the IR line by line looking for QIS function calls
    for line in ir_text.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with(';') {
            continue;
        }

        // Look for QIS function calls
        if let Some(op) = parse_qis_call(line, &mut interface, &mut qubit_map, &mut result_map)? {
            interface.queue_operation(op);
        }
    }

    Ok(interface)
}

/// Parse a single QIS function call from LLVM IR
fn parse_qis_call(
    line: &str,
    interface: &mut QisInterface,
    qubit_map: &mut HashMap<i64, usize>,
    result_map: &mut HashMap<i64, usize>,
) -> Result<Option<Operation>, PecosError> {
    // Look for call instructions to QIS functions
    if !line.contains("call") || !line.contains("@__quantum__qis__") {
        return Ok(None);
    }

    // Extract function name and arguments
    // Pattern: call <return_type> @__quantum__qis__<gate>__body(<args>)

    // Single-qubit gates
    if line.contains("@__quantum__qis__h__body") {
        let qubit = parse_qubit_arg(line, interface, qubit_map)?;
        return Ok(Some(QuantumOp::H(qubit).into()));
    }

    if line.contains("@__quantum__qis__x__body") {
        let qubit = parse_qubit_arg(line, interface, qubit_map)?;
        return Ok(Some(QuantumOp::X(qubit).into()));
    }

    if line.contains("@__quantum__qis__y__body") {
        let qubit = parse_qubit_arg(line, interface, qubit_map)?;
        return Ok(Some(QuantumOp::Y(qubit).into()));
    }

    if line.contains("@__quantum__qis__z__body") {
        let qubit = parse_qubit_arg(line, interface, qubit_map)?;
        return Ok(Some(QuantumOp::Z(qubit).into()));
    }

    if line.contains("@__quantum__qis__s__body") {
        let qubit = parse_qubit_arg(line, interface, qubit_map)?;
        return Ok(Some(QuantumOp::S(qubit).into()));
    }

    if line.contains("@__quantum__qis__t__body") {
        let qubit = parse_qubit_arg(line, interface, qubit_map)?;
        return Ok(Some(QuantumOp::T(qubit).into()));
    }

    // Two-qubit gates
    if line.contains("@__quantum__qis__cx__body") || line.contains("@__quantum__qis__cnot__body") {
        let (control, target) = parse_two_qubit_args(line, interface, qubit_map)?;
        return Ok(Some(QuantumOp::CX(control, target).into()));
    }

    if line.contains("@__quantum__qis__cz__body") {
        let (control, target) = parse_two_qubit_args(line, interface, qubit_map)?;
        return Ok(Some(QuantumOp::CZ(control, target).into()));
    }

    // Measurement
    if line.contains("@__quantum__qis__m__body") || line.contains("@__quantum__qis__measure__body") {
        let (qubit, result) = parse_measurement_args(line, interface, qubit_map, result_map)?;
        return Ok(Some(QuantumOp::Measure(qubit, result).into()));
    }

    // If we reach here, it's a QIS call we don't recognize
    log::warn!("Unrecognized QIS call: {}", line);
    Ok(None)
}

/// Parse a single qubit argument from a QIS call
fn parse_qubit_arg(
    line: &str,
    interface: &mut QisInterface,
    qubit_map: &mut HashMap<i64, usize>
) -> Result<usize, PecosError> {
    // Look for pattern like "i64 123" or "i64 %var"
    let re = regex::Regex::new(r"i64\s+(\d+)").map_err(|e|
        PecosError::Generic(format!("Regex error: {}", e))
    )?;

    if let Some(caps) = re.captures(line) {
        let llvm_qubit_id: i64 = caps[1].parse().map_err(|e|
            PecosError::Generic(format!("Failed to parse qubit ID: {}", e))
        )?;

        // Map LLVM qubit ID to interface qubit ID
        let interface_qubit_id = *qubit_map.entry(llvm_qubit_id)
            .or_insert_with(|| interface.allocate_qubit());

        Ok(interface_qubit_id)
    } else {
        Err(PecosError::Generic(format!("Could not parse qubit argument from: {}", line)))
    }
}

/// Parse two qubit arguments from a two-qubit gate call
fn parse_two_qubit_args(
    line: &str,
    interface: &mut QisInterface,
    qubit_map: &mut HashMap<i64, usize>,
) -> Result<(usize, usize), PecosError> {
    // Look for pattern like "i64 123, i64 456"
    let re = regex::Regex::new(r"i64\s+(\d+),\s*i64\s+(\d+)").map_err(|e|
        PecosError::Generic(format!("Regex error: {}", e))
    )?;

    if let Some(caps) = re.captures(line) {
        let llvm_qubit1: i64 = caps[1].parse().map_err(|e|
            PecosError::Generic(format!("Failed to parse first qubit ID: {}", e))
        )?;
        let llvm_qubit2: i64 = caps[2].parse().map_err(|e|
            PecosError::Generic(format!("Failed to parse second qubit ID: {}", e))
        )?;

        let qubit1 = *qubit_map.entry(llvm_qubit1)
            .or_insert_with(|| interface.allocate_qubit());
        let qubit2 = *qubit_map.entry(llvm_qubit2)
            .or_insert_with(|| interface.allocate_qubit());

        Ok((qubit1, qubit2))
    } else {
        Err(PecosError::Generic(format!("Could not parse two-qubit arguments from: {}", line)))
    }
}

/// Parse measurement arguments (qubit and result)
fn parse_measurement_args(
    line: &str,
    interface: &mut QisInterface,
    qubit_map: &mut HashMap<i64, usize>,
    result_map: &mut HashMap<i64, usize>,
) -> Result<(usize, usize), PecosError> {
    // Look for pattern like "i64 123, i64 456" where first is qubit, second is result
    let re = regex::Regex::new(r"i64\s+(\d+),\s*i64\s+(\d+)").map_err(|e|
        PecosError::Generic(format!("Regex error: {}", e))
    )?;

    if let Some(caps) = re.captures(line) {
        let llvm_qubit_id: i64 = caps[1].parse().map_err(|e|
            PecosError::Generic(format!("Failed to parse qubit ID: {}", e))
        )?;
        let llvm_result_id: i64 = caps[2].parse().map_err(|e|
            PecosError::Generic(format!("Failed to parse result ID: {}", e))
        )?;

        let qubit_id = *qubit_map.entry(llvm_qubit_id)
            .or_insert_with(|| interface.allocate_qubit());
        let result_id = *result_map.entry(llvm_result_id)
            .or_insert_with(|| interface.allocate_result());

        Ok((qubit_id, result_id))
    } else {
        Err(PecosError::Generic(format!("Could not parse measurement arguments from: {}", line)))
    }
}