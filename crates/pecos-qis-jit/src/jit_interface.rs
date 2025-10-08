//! JIT interface implementation
//!
//! This module implements the `QisInterface` trait using LLVM JIT compilation.
//! Each interface instance is completely isolated with its own state and FFI symbols.

use inkwell::OptimizationLevel;
use inkwell::context::Context;
use inkwell::execution_engine::ExecutionEngine;
use inkwell::memory_buffer::MemoryBuffer;
use inkwell::targets::{InitializationConfig, Target};
use pecos_qis_core::qis_interface::{InterfaceError, ProgramFormat, QisInterface};
use pecos_qis_ffi::{Operation, OperationCollector, QuantumOp};
use rand::random;
use std::collections::HashMap;

/// JIT-based QIS interface implementation
///
/// This interface uses LLVM JIT compilation to execute quantum programs
/// and collect the operations. Each instance is completely isolated with
/// its own operation collection and state. LLVM contexts and execution
/// engines are created fresh for each execution to ensure safety.
pub struct QisJitInterface {
    /// The loaded program (if any)
    program: Option<Vec<u8>>,

    /// The program format
    format: ProgramFormat,

    /// Collected operations (instance-owned state)
    operations: OperationCollector,

    /// Instance measurements (no global state)
    measurements: HashMap<usize, bool>,

    /// Metadata
    metadata: HashMap<String, String>,

    /// Unique instance ID for symbol generation
    instance_id: usize,
}

impl QisJitInterface {
    /// Helper to convert i64 to usize safely
    #[inline]
    fn i64_to_usize(value: i64) -> usize {
        usize::try_from(value).expect("Invalid ID: value must be non-negative and fit in usize")
    }

    /// Create a new JIT interface
    #[must_use]
    pub fn new() -> Self {
        // Generate unique instance ID - safe cast since u32 always fits in usize
        #[allow(clippy::cast_lossless)]
        let instance_id = random::<u32>() as usize;

        Self {
            program: None,
            format: ProgramFormat::LlvmIrText,
            operations: OperationCollector::new(),
            measurements: HashMap::new(),
            metadata: HashMap::new(),
            instance_id,
        }
    }

    /// Initialize LLVM (static initialization, safe to call multiple times)
    fn ensure_llvm_initialized() {
        static LLVM_INIT: std::sync::Once = std::sync::Once::new();
        LLVM_INIT.call_once(|| {
            Target::initialize_all(&InitializationConfig::default());
            let _ = Target::initialize_native(&InitializationConfig::default());
        });
    }

    /// Create instance-specific FFI symbol mappings
    fn create_instance_symbols() -> HashMap<String, *const ()> {
        let mut symbols = HashMap::new();

        // Map standard QIS symbols to instance-specific implementations
        symbols.insert(
            "__quantum__qis__h__body".to_string(),
            Self::ffi_h_body as *const (),
        );
        symbols.insert(
            "__quantum__qis__x__body".to_string(),
            Self::ffi_x_body as *const (),
        );
        symbols.insert(
            "__quantum__qis__y__body".to_string(),
            Self::ffi_y_body as *const (),
        );
        symbols.insert(
            "__quantum__qis__z__body".to_string(),
            Self::ffi_z_body as *const (),
        );
        symbols.insert(
            "__quantum__qis__s__body".to_string(),
            Self::ffi_s_body as *const (),
        );
        symbols.insert(
            "__quantum__qis__t__body".to_string(),
            Self::ffi_t_body as *const (),
        );
        symbols.insert(
            "__quantum__qis__cx__body".to_string(),
            Self::ffi_cx_body as *const (),
        );

        // Map rotation gates
        symbols.insert(
            "__quantum__qis__rx__body".to_string(),
            Self::ffi_rx_body as *const (),
        );
        symbols.insert(
            "__quantum__qis__ry__body".to_string(),
            Self::ffi_ry_body as *const (),
        );
        symbols.insert(
            "__quantum__qis__rz__body".to_string(),
            Self::ffi_rz_body as *const (),
        );

        // Map two-qubit ZZ gate
        symbols.insert(
            "__quantum__qis__zz__body".to_string(),
            Self::ffi_zz_body as *const (),
        );

        // Map allocation and measurement functions
        symbols.insert(
            "__quantum__rt__qubit_allocate".to_string(),
            Self::ffi_qubit_allocate as *const (),
        );
        symbols.insert(
            "__quantum__rt__qubit_release".to_string(),
            Self::ffi_qubit_release as *const (),
        );
        symbols.insert(
            "__quantum__qis__m__body".to_string(),
            Self::ffi_measure as *const (),
        );
        symbols.insert(
            "__quantum__rt__result_allocate".to_string(),
            Self::ffi_result_allocate as *const (),
        );
        symbols.insert(
            "__quantum__rt__result_record_output".to_string(),
            Self::ffi_result_record_output as *const (),
        );

        symbols
    }

    /// Instance-specific FFI implementations (no global state)
    unsafe extern "C" fn ffi_h_body(instance_ptr: *mut Self, qubit: i64) {
        log::debug!("ffi_h_body called with instance_ptr={instance_ptr:p}, qubit={qubit}");
        if !instance_ptr.is_null() {
            let instance = unsafe { &mut *instance_ptr };
            let qubit_id = Self::i64_to_usize(qubit);
            instance
                .operations
                .queue_operation(QuantumOp::H(qubit_id).into());
            log::debug!("H operation queued for qubit {qubit_id}");
        }
    }

    unsafe extern "C" fn ffi_x_body(instance_ptr: *mut Self, qubit: i64) {
        log::debug!("ffi_x_body called with instance_ptr={instance_ptr:p}, qubit={qubit}");
        if !instance_ptr.is_null() {
            let instance = unsafe { &mut *instance_ptr };
            let qubit_id = Self::i64_to_usize(qubit);
            instance
                .operations
                .queue_operation(QuantumOp::X(qubit_id).into());
            log::debug!("X operation queued for qubit {qubit_id}");
        }
    }

    unsafe extern "C" fn ffi_y_body(instance_ptr: *mut Self, qubit: i64) {
        if !instance_ptr.is_null() {
            let instance = unsafe { &mut *instance_ptr };
            let qubit_id = Self::i64_to_usize(qubit);
            instance
                .operations
                .queue_operation(QuantumOp::Y(qubit_id).into());
        }
    }

    unsafe extern "C" fn ffi_z_body(instance_ptr: *mut Self, qubit: i64) {
        if !instance_ptr.is_null() {
            let instance = unsafe { &mut *instance_ptr };
            let qubit_id = Self::i64_to_usize(qubit);
            instance
                .operations
                .queue_operation(QuantumOp::Z(qubit_id).into());
        }
    }

    unsafe extern "C" fn ffi_s_body(instance_ptr: *mut Self, qubit: i64) {
        if !instance_ptr.is_null() {
            let instance = unsafe { &mut *instance_ptr };
            let qubit_id = Self::i64_to_usize(qubit);
            instance
                .operations
                .queue_operation(QuantumOp::S(qubit_id).into());
        }
    }

    unsafe extern "C" fn ffi_t_body(instance_ptr: *mut Self, qubit: i64) {
        if !instance_ptr.is_null() {
            let instance = unsafe { &mut *instance_ptr };
            let qubit_id = Self::i64_to_usize(qubit);
            instance
                .operations
                .queue_operation(QuantumOp::T(qubit_id).into());
        }
    }

    unsafe extern "C" fn ffi_cx_body(instance_ptr: *mut Self, control: i64, target: i64) {
        if !instance_ptr.is_null() {
            let instance = unsafe { &mut *instance_ptr };
            let control_id = Self::i64_to_usize(control);
            let target_id = Self::i64_to_usize(target);
            instance
                .operations
                .queue_operation(QuantumOp::CX(control_id, target_id).into());
        }
    }

    // Rotation gate FFI functions
    unsafe extern "C" fn ffi_rx_body(instance_ptr: *mut Self, angle: f64, qubit: i64) {
        if !instance_ptr.is_null() {
            let instance = unsafe { &mut *instance_ptr };
            let qubit_id = Self::i64_to_usize(qubit);
            instance
                .operations
                .queue_operation(QuantumOp::RX(angle, qubit_id).into());
        }
    }

    unsafe extern "C" fn ffi_ry_body(instance_ptr: *mut Self, angle: f64, qubit: i64) {
        if !instance_ptr.is_null() {
            let instance = unsafe { &mut *instance_ptr };
            let qubit_id = Self::i64_to_usize(qubit);
            instance
                .operations
                .queue_operation(QuantumOp::RY(angle, qubit_id).into());
        }
    }

    unsafe extern "C" fn ffi_rz_body(instance_ptr: *mut Self, angle: f64, qubit: i64) {
        if !instance_ptr.is_null() {
            let instance = unsafe { &mut *instance_ptr };
            let qubit_id = Self::i64_to_usize(qubit);
            instance
                .operations
                .queue_operation(QuantumOp::RZ(angle, qubit_id).into());
        }
    }

    unsafe extern "C" fn ffi_zz_body(instance_ptr: *mut Self, qubit1: i64, qubit2: i64) {
        if !instance_ptr.is_null() {
            let instance = unsafe { &mut *instance_ptr };
            let qubit1_id = Self::i64_to_usize(qubit1);
            let qubit2_id = Self::i64_to_usize(qubit2);
            instance
                .operations
                .queue_operation(QuantumOp::ZZ(qubit1_id, qubit2_id).into());
        }
    }

    // Allocation and measurement FFI functions
    unsafe extern "C" fn ffi_qubit_allocate(instance_ptr: *mut Self) -> i64 {
        if instance_ptr.is_null() {
            0
        } else {
            let instance = unsafe { &mut *instance_ptr };
            let id = instance.operations.allocate_qubit();
            instance
                .operations
                .queue_operation(Operation::AllocateQubit { id });
            i64::try_from(id).expect("Qubit ID too large for i64")
        }
    }

    unsafe extern "C" fn ffi_qubit_release(instance_ptr: *mut Self, qubit: i64) {
        if !instance_ptr.is_null() {
            let instance = unsafe { &mut *instance_ptr };
            let qubit_id = Self::i64_to_usize(qubit);
            instance
                .operations
                .queue_operation(Operation::ReleaseQubit { id: qubit_id });
        }
    }

    unsafe extern "C" fn ffi_measure(instance_ptr: *mut Self, qubit: i64, result: i64) -> i32 {
        if instance_ptr.is_null() {
            0
        } else {
            let instance = unsafe { &mut *instance_ptr };
            let qubit_id = Self::i64_to_usize(qubit);
            let result_id = Self::i64_to_usize(result);

            instance
                .operations
                .queue_operation(QuantumOp::Measure(qubit_id, result_id).into());

            // Return measurement result from instance state (not global state)
            instance
                .measurements
                .get(&result_id)
                .map_or(0, |&b| i32::from(b))
        }
    }

    unsafe extern "C" fn ffi_result_allocate(instance_ptr: *mut Self) -> i64 {
        if instance_ptr.is_null() {
            0
        } else {
            let instance = unsafe { &mut *instance_ptr };
            let id = instance.operations.allocate_result();
            instance
                .operations
                .queue_operation(Operation::AllocateResult { id });
            i64::try_from(id).expect("Result ID too large for i64")
        }
    }

    unsafe extern "C" fn ffi_result_record_output(
        instance_ptr: *mut Self,
        result_id: i64,
        _register_name: *const i8,
    ) {
        if !instance_ptr.is_null() {
            let _instance = unsafe { &mut *instance_ptr };
            let _result_id = Self::i64_to_usize(result_id);
            // This is a no-op for collection mode - we're just collecting operations
            // The actual result recording will be handled by the runtime during execution
            // This function just needs to exist so LLVM doesn't crash
        }
    }

    /// Execute LLVM IR with complete instance isolation
    fn execute_isolated_llvm_ir(&mut self, llvm_ir: &str) -> Result<(), InterfaceError> {
        log::debug!("[JIT] execute_isolated_llvm_ir called");
        log::debug!("[JIT] LLVM IR length: {} bytes", llvm_ir.len());
        Self::ensure_llvm_initialized();

        // Clear previous state
        self.operations = OperationCollector::new();

        // Transform LLVM IR to use instance-specific calls
        let transformed_ir = self.transform_llvm_ir_for_instance(llvm_ir);

        // Create memory buffer from transformed LLVM IR
        let memory_buffer = MemoryBuffer::create_from_memory_range_copy(
            transformed_ir.as_bytes(),
            &format!("qis_ir_{}", self.instance_id),
        );

        // Create fresh context for this instance
        log::debug!("Creating LLVM context for instance {}", self.instance_id);
        let context = Context::create();

        // Parse LLVM IR into a module
        log::debug!("Parsing LLVM IR into module");
        let module = context
            .create_module_from_ir(memory_buffer)
            .map_err(|e| InterfaceError::ExecutionError(format!("Failed to parse LLVM IR: {e}")))?;

        // Create execution engine
        log::debug!("Creating JIT execution engine");
        let execution_engine = module
            .create_jit_execution_engine(OptimizationLevel::None)
            .map_err(|e| {
                InterfaceError::ExecutionError(format!("Failed to create execution engine: {e}"))
            })?;

        // Map instance-specific FFI symbols
        log::debug!("Mapping instance-specific FFI symbols");
        Self::map_instance_symbols(&execution_engine, &module);
        log::debug!("Symbol mapping completed");

        // Find entry point function - try qmain first (Selene convention), then main (standard QIR)
        let main_function = module
            .get_function("qmain")
            .or_else(|| module.get_function("main"))
            .ok_or_else(|| {
                InterfaceError::ExecutionError(
                    "No entry point function (qmain or main) found in LLVM IR".to_string(),
                )
            })?;

        // Execute entry point function (calls our instance-specific FFI functions)
        log::debug!(
            "About to execute entry point function: {}",
            main_function.get_name().to_str().unwrap_or("unknown")
        );

        // Execute the entry point function
        unsafe {
            let _result = execution_engine.run_function(main_function, &[]);
        }
        log::debug!("Entry point function execution completed successfully");

        Ok(())
    }

    /// Transform LLVM IR to use instance-specific function calls
    #[allow(clippy::similar_names)] // Quantum gate names are intentionally similar (rx, ry, rz, etc.)
    #[allow(clippy::too_many_lines)] // LLVM IR transformation requires comprehensive pattern matching for all gate types
    fn transform_llvm_ir_for_instance(&self, llvm_ir: &str) -> String {
        let instance_ptr = std::ptr::from_ref::<Self>(self) as usize;

        // Transform FFI calls to include instance pointer as first parameter
        let mut transformed = llvm_ir.to_string();

        // Pre-format strings to avoid lifetime issues
        let h_call = format!(
            "call void @__quantum__qis__h__body(i8* inttoptr (i64 {instance_ptr} to i8*), i64"
        );
        let x_call = format!(
            "call void @__quantum__qis__x__body(i8* inttoptr (i64 {instance_ptr} to i8*), i64"
        );
        let y_call = format!(
            "call void @__quantum__qis__y__body(i8* inttoptr (i64 {instance_ptr} to i8*), i64"
        );
        let z_call = format!(
            "call void @__quantum__qis__z__body(i8* inttoptr (i64 {instance_ptr} to i8*), i64"
        );
        let s_call = format!(
            "call void @__quantum__qis__s__body(i8* inttoptr (i64 {instance_ptr} to i8*), i64"
        );
        let t_call = format!(
            "call void @__quantum__qis__t__body(i8* inttoptr (i64 {instance_ptr} to i8*), i64"
        );
        let cx_call = format!(
            "call void @__quantum__qis__cx__body(i8* inttoptr (i64 {instance_ptr} to i8*), i64"
        );
        let rx_call = format!(
            "call void @__quantum__qis__rx__body(i8* inttoptr (i64 {instance_ptr} to i8*), double"
        );
        let ry_call = format!(
            "call void @__quantum__qis__ry__body(i8* inttoptr (i64 {instance_ptr} to i8*), double"
        );
        let rz_call = format!(
            "call void @__quantum__qis__rz__body(i8* inttoptr (i64 {instance_ptr} to i8*), double"
        );
        let zz_call = format!(
            "call void @__quantum__qis__zz__body(i8* inttoptr (i64 {instance_ptr} to i8*), i64"
        );
        let qalloc_call = format!(
            "call i64 @__quantum__rt__qubit_allocate(i8* inttoptr (i64 {instance_ptr} to i8*))"
        );
        let qfree_call = format!(
            "call void @__quantum__rt__qubit_release(i8* inttoptr (i64 {instance_ptr} to i8*), i64"
        );
        let ralloc_call = format!(
            "call i64 @__quantum__rt__result_allocate(i8* inttoptr (i64 {instance_ptr} to i8*))"
        );
        let rrecord_call = format!(
            "call void @__quantum__rt__result_record_output(i8* inttoptr (i64 {instance_ptr} to i8*), i64"
        );

        // Apply transforms
        transformed = transformed.replace("call void @__quantum__qis__h__body(i64", &h_call);
        transformed = transformed.replace("call void @__quantum__qis__x__body(i64", &x_call);
        transformed = transformed.replace("call void @__quantum__qis__y__body(i64", &y_call);
        transformed = transformed.replace("call void @__quantum__qis__z__body(i64", &z_call);
        transformed = transformed.replace("call void @__quantum__qis__s__body(i64", &s_call);
        transformed = transformed.replace("call void @__quantum__qis__t__body(i64", &t_call);
        transformed = transformed.replace("call void @__quantum__qis__cx__body(i64", &cx_call);
        transformed = transformed.replace("call void @__quantum__qis__rx__body(double", &rx_call);
        transformed = transformed.replace("call void @__quantum__qis__ry__body(double", &ry_call);
        transformed = transformed.replace("call void @__quantum__qis__rz__body(double", &rz_call);
        transformed = transformed.replace("call void @__quantum__qis__zz__body(i64", &zz_call);
        transformed =
            transformed.replace("call i64 @__quantum__rt__qubit_allocate()", &qalloc_call);
        transformed =
            transformed.replace("call void @__quantum__rt__qubit_release(i64", &qfree_call);
        // Handle measurement calls with two parameters (qubit, result)
        let measure_pattern = "call i32 @__quantum__qis__m__body(i64";
        let measure_replacement = format!(
            "call i32 @__quantum__qis__m__body(i8* inttoptr (i64 {instance_ptr} to i8*), i64"
        );
        transformed = transformed.replace(measure_pattern, &measure_replacement);
        transformed =
            transformed.replace("call i64 @__quantum__rt__result_allocate()", &ralloc_call);
        transformed = transformed.replace(
            "call void @__quantum__rt__result_record_output(i64",
            &rrecord_call,
        );

        // Update function declarations to include instance pointer
        transformed = transformed.replace(
            "declare void @__quantum__qis__h__body(i64)",
            "declare void @__quantum__qis__h__body(i8*, i64)",
        );
        transformed = transformed.replace(
            "declare void @__quantum__qis__x__body(i64)",
            "declare void @__quantum__qis__x__body(i8*, i64)",
        );
        transformed = transformed.replace(
            "declare void @__quantum__qis__y__body(i64)",
            "declare void @__quantum__qis__y__body(i8*, i64)",
        );
        transformed = transformed.replace(
            "declare void @__quantum__qis__z__body(i64)",
            "declare void @__quantum__qis__z__body(i8*, i64)",
        );
        transformed = transformed.replace(
            "declare void @__quantum__qis__s__body(i64)",
            "declare void @__quantum__qis__s__body(i8*, i64)",
        );
        transformed = transformed.replace(
            "declare void @__quantum__qis__t__body(i64)",
            "declare void @__quantum__qis__t__body(i8*, i64)",
        );
        transformed = transformed.replace(
            "declare void @__quantum__qis__cx__body(i64, i64)",
            "declare void @__quantum__qis__cx__body(i8*, i64, i64)",
        );
        transformed = transformed.replace(
            "declare void @__quantum__qis__rx__body(double, i64)",
            "declare void @__quantum__qis__rx__body(i8*, double, i64)",
        );
        transformed = transformed.replace(
            "declare void @__quantum__qis__ry__body(double, i64)",
            "declare void @__quantum__qis__ry__body(i8*, double, i64)",
        );
        transformed = transformed.replace(
            "declare void @__quantum__qis__rz__body(double, i64)",
            "declare void @__quantum__qis__rz__body(i8*, double, i64)",
        );
        transformed = transformed.replace(
            "declare void @__quantum__qis__zz__body(i64, i64)",
            "declare void @__quantum__qis__zz__body(i8*, i64, i64)",
        );
        transformed = transformed.replace(
            "declare i64 @__quantum__rt__qubit_allocate()",
            "declare i64 @__quantum__rt__qubit_allocate(i8*)",
        );
        transformed = transformed.replace(
            "declare void @__quantum__rt__qubit_release(i64)",
            "declare void @__quantum__rt__qubit_release(i8*, i64)",
        );
        transformed = transformed.replace(
            "declare i32 @__quantum__qis__m__body(i64, i64)",
            "declare i32 @__quantum__qis__m__body(i8*, i64, i64)",
        );
        transformed = transformed.replace(
            "declare i64 @__quantum__rt__result_allocate()",
            "declare i64 @__quantum__rt__result_allocate(i8*)",
        );
        transformed = transformed.replace(
            "declare void @__quantum__rt__result_record_output(i64, i8*)",
            "declare void @__quantum__rt__result_record_output(i8*, i64, i8*)",
        );

        transformed
    }

    /// Map instance-specific symbols to the execution engine
    fn map_instance_symbols(engine: &ExecutionEngine, module: &inkwell::module::Module) {
        let symbols = Self::create_instance_symbols();

        // Map each symbol to its function pointer in the execution engine
        for (name, func_ptr) in symbols {
            if let Some(function) = module.get_function(&name) {
                log::debug!("Mapping symbol: {name} -> {func_ptr:p}");
                engine.add_global_mapping(&function, func_ptr as usize);
            } else {
                log::warn!("Symbol {name} not found in module - this may cause execution failures");
            }
        }
    }

    /// Convert program bytes to LLVM IR text
    fn get_llvm_ir(&self) -> Result<String, InterfaceError> {
        let program_bytes = self
            .program
            .as_ref()
            .ok_or_else(|| InterfaceError::Other("No program loaded".to_string()))?;

        match self.format {
            ProgramFormat::LlvmIrText => String::from_utf8(program_bytes.clone())
                .map_err(|e| InterfaceError::Other(format!("Invalid UTF-8 in LLVM IR: {e}"))),
            ProgramFormat::LlvmBitcode | ProgramFormat::QisBitcode => {
                // Would need to use llvm-dis to convert bitcode to text
                // For now, return an error
                Err(InterfaceError::Other(
                    "JIT interface currently only supports LLVM IR text. \
                     Convert bitcode to text using llvm-dis first."
                        .to_string(),
                ))
            }
            ProgramFormat::HugrBytes => {
                // Would need to compile HUGR first
                Err(InterfaceError::Other(
                    "JIT interface requires HUGR to be compiled to LLVM IR first".to_string(),
                ))
            }
        }
    }
}

impl QisInterface for QisJitInterface {
    fn load_program(
        &mut self,
        program_bytes: &[u8],
        format: ProgramFormat,
    ) -> Result<(), InterfaceError> {
        self.program = Some(program_bytes.to_vec());
        self.format = format;

        // Validate that we can handle this format
        self.get_llvm_ir()?;

        self.metadata
            .insert("format".to_string(), format!("{format:?}"));
        self.metadata
            .insert("program_size".to_string(), program_bytes.len().to_string());

        Ok(())
    }

    fn collect_operations(&mut self) -> Result<OperationCollector, InterfaceError> {
        let llvm_ir = self.get_llvm_ir()?;

        // Execute using isolated instance-specific compilation
        self.execute_isolated_llvm_ir(&llvm_ir)?;

        // Update metadata
        self.metadata.insert(
            "operations_collected".to_string(),
            self.operations.operations.len().to_string(),
        );
        self.metadata.insert(
            "qubits_allocated".to_string(),
            self.operations.allocated_qubits.len().to_string(),
        );

        log::debug!(
            "[JIT] Collected {} operations",
            self.operations.operations.len()
        );
        log::debug!("[JIT] Operations: {:?}", self.operations.operations);
        log::debug!("Allocated qubits: {:?}", self.operations.allocated_qubits);

        Ok(self.operations.clone())
    }

    fn execute_with_measurements(
        &mut self,
        measurements: HashMap<usize, bool>,
    ) -> Result<OperationCollector, InterfaceError> {
        let llvm_ir = self.get_llvm_ir()?;

        // Store measurements in instance state (no global state)
        self.measurements = measurements;

        // Execute with instance-specific measurements
        self.execute_isolated_llvm_ir(&llvm_ir)?;

        Ok(self.operations.clone())
    }

    fn metadata(&self) -> HashMap<String, String> {
        self.metadata.clone()
    }

    fn name(&self) -> &'static str {
        "LLVM JIT"
    }

    fn reset(&mut self) -> Result<(), InterfaceError> {
        // Reset only instance state, no global state
        self.operations = OperationCollector::new();
        self.measurements.clear();
        Ok(())
    }
}

impl Default for QisJitInterface {
    fn default() -> Self {
        Self::new()
    }
}
