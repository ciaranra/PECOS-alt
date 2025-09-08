/// Template for generating Selene simulator runtime plugins
///
/// This module provides a clean way to generate the plugin code without
/// format string escaping issues.
use std::fmt::Write as FmtWrite;

/// Generate the complete simulator plugin code
pub fn generate_simulator_plugin_code(entry_point: &str, _enable_metrics: bool) -> String {
    let mut code = String::with_capacity(16 * 1024); // Pre-allocate ~16KB

    // Add imports
    code.push_str(IMPORTS);
    code.push('\n');

    // Add global state
    code.push_str(GLOBAL_STATE);
    code.push('\n');

    // Add entry point extern declaration
    writeln!(&mut code, "// Entry point function from LLVM").unwrap();
    writeln!(&mut code, "extern \"C\" {{ fn {}(); }}", entry_point).unwrap();
    code.push('\n');

    // Add quantum intrinsics
    code.push_str(QUANTUM_INTRINSICS);
    code.push('\n');

    // Add SimulatorRuntimePlugin struct
    code.push_str(SIMULATOR_RUNTIME_PLUGIN_STRUCT);
    code.push('\n');

    // Add SimulatorRuntimePlugin implementation with entry point
    add_simulator_impl(&mut code, entry_point);
    code.push('\n');

    // Add RuntimeInterface implementation
    code.push_str(RUNTIME_INTERFACE_IMPL);
    code.push('\n');

    // Add factory and export
    code.push_str(FACTORY_AND_EXPORT);

    code
}

const IMPORTS: &str = r#"use std::collections::VecDeque;
use anyhow::{Result, bail};
use selene_core::{
    export_runtime_plugin,
    runtime::{BatchOperation, Operation, RuntimeInterface, interface::RuntimeInterfaceFactory},
    utils::MetricValue,
    encoder::{OutputStream, OutputStreamError},
};
use std::sync::Mutex;"#;

const GLOBAL_STATE: &str = r#"// Global state to track operations that will be converted to byte messages
static OPERATION_QUEUE: Mutex<VecDeque<Operation>> = Mutex::new(VecDeque::new());
static MEASUREMENT_COUNTER: Mutex<u64> = Mutex::new(0);"#;

const QUANTUM_INTRINSICS: &str = r#"// Override the quantum intrinsics to capture operations
#[no_mangle]
pub unsafe extern "C" fn __quantum__qis__h__body(qubit: i64) {
    let mut queue = OPERATION_QUEUE.lock().unwrap();
    // H gate is RXY(π, 0)
    queue.push_back(Operation::RXYGate {
        qubit_id: qubit as u64,
        theta: std::f64::consts::PI,
        phi: 0.0,
    });
}

#[no_mangle]
pub unsafe extern "C" fn __quantum__qis__x__body(qubit: i64) {
    let mut queue = OPERATION_QUEUE.lock().unwrap();
    // X gate is RXY(π, π)
    queue.push_back(Operation::RXYGate {
        qubit_id: qubit as u64,
        theta: std::f64::consts::PI,
        phi: std::f64::consts::PI,
    });
}

#[no_mangle]
pub unsafe extern "C" fn __quantum__qis__y__body(qubit: i64) {
    let mut queue = OPERATION_QUEUE.lock().unwrap();
    // Y gate is RXY(π, π/2)
    queue.push_back(Operation::RXYGate {
        qubit_id: qubit as u64,
        theta: std::f64::consts::PI,
        phi: std::f64::consts::PI / 2.0,
    });
}

#[no_mangle]
pub unsafe extern "C" fn __quantum__qis__z__body(qubit: i64) {
    let mut queue = OPERATION_QUEUE.lock().unwrap();
    // Z gate is RZ(π)
    queue.push_back(Operation::RZGate {
        qubit_id: qubit as u64,
        theta: std::f64::consts::PI,
    });
}

#[no_mangle]
pub unsafe extern "C" fn __quantum__qis__cx__body(control: i64, target: i64) {
    let mut queue = OPERATION_QUEUE.lock().unwrap();
    // CNOT is RZZ(π) with some single qubit corrections
    queue.push_back(Operation::RZZGate {
        qubit_id_1: control as u64,
        qubit_id_2: target as u64,
        theta: std::f64::consts::PI,
    });
}

#[no_mangle]
pub unsafe extern "C" fn __quantum__qis__m__body(qubit: i64, result_id: i64) -> i32 {
    let mut queue = OPERATION_QUEUE.lock().unwrap();
    let mut counter = MEASUREMENT_COUNTER.lock().unwrap();

    let measurement_id = *counter;
    *counter += 1;

    queue.push_back(Operation::Measure {
        qubit_id: qubit as u64,
        result_id: measurement_id,
    });

    // Return a placeholder value - actual results come from PECOS
    0
}

#[no_mangle]
pub unsafe extern "C" fn __quantum__qis__rz__body(theta: f64, qubit: i64) {
    let mut queue = OPERATION_QUEUE.lock().unwrap();
    queue.push_back(Operation::RZGate {
        qubit_id: qubit as u64,
        theta,
    });
}

#[no_mangle]
pub unsafe extern "C" fn __quantum__qis__ry__body(theta: f64, qubit: i64) {
    let mut queue = OPERATION_QUEUE.lock().unwrap();
    // RY is RXY(theta, π/2)
    queue.push_back(Operation::RXYGate {
        qubit_id: qubit as u64,
        theta,
        phi: std::f64::consts::PI / 2.0,
    });
}

#[no_mangle]
pub unsafe extern "C" fn __quantum__qis__rx__body(theta: f64, qubit: i64) {
    let mut queue = OPERATION_QUEUE.lock().unwrap();
    // RX is RXY(theta, 0)
    queue.push_back(Operation::RXYGate {
        qubit_id: qubit as u64,
        theta,
        phi: 0.0,
    });
}"#;

const SIMULATOR_RUNTIME_PLUGIN_STRUCT: &str = r#"struct SimulatorRuntimePlugin {
    measurements: Vec<bool>,
    start: selene_core::time::Instant,
    current_result_id: u64,
}"#;

fn add_simulator_impl(code: &mut String, entry_point: &str) {
    writeln!(code, "impl SimulatorRuntimePlugin {{").unwrap();
    writeln!(
        code,
        "    pub fn new(start: selene_core::time::Instant) -> Self {{"
    )
    .unwrap();
    writeln!(code, "        Self {{").unwrap();
    writeln!(code, "            measurements: Vec::new(),").unwrap();
    writeln!(code, "            start,").unwrap();
    writeln!(code, "            current_result_id: 0,").unwrap();
    writeln!(code, "        }}").unwrap();
    writeln!(code, "    }}").unwrap();
    writeln!(code, "    ").unwrap();
    writeln!(code, "    fn execute_llvm_program(&mut self) {{").unwrap();
    writeln!(code, "        // Clear the operation queue").unwrap();
    writeln!(code, "        OPERATION_QUEUE.lock().unwrap().clear();").unwrap();
    writeln!(code, "        *MEASUREMENT_COUNTER.lock().unwrap() = 0;").unwrap();
    writeln!(code, "        ").unwrap();
    writeln!(code, "        // Call the entry point function").unwrap();
    writeln!(
        code,
        "        // This will populate the operation queue via our overridden intrinsics"
    )
    .unwrap();
    writeln!(code, "        unsafe {{").unwrap();
    writeln!(code, "            {}();", entry_point).unwrap();
    writeln!(code, "        }}").unwrap();
    writeln!(code, "    }}").unwrap();
    writeln!(code, "}}").unwrap();
}

const RUNTIME_INTERFACE_IMPL: &str = r#"impl RuntimeInterface for SimulatorRuntimePlugin {
    fn exit(&mut self) -> Result<()> {
        OPERATION_QUEUE.lock().unwrap().clear();
        self.measurements.clear();
        Ok(())
    }

    fn get_next_operations(&mut self) -> Result<Option<BatchOperation>> {
        // Execute LLVM program if queue is empty
        if OPERATION_QUEUE.lock().unwrap().is_empty() {
            self.execute_llvm_program();
        }

        // Get all operations from the queue
        let mut queue = OPERATION_QUEUE.lock().unwrap();
        if queue.is_empty() {
            return Ok(None);
        }

        let operations: Vec<Operation> = queue.drain(..).collect();

        Ok(Some(BatchOperation::new(
            operations,
            self.start,
            Default::default(),
        )))
    }

    fn shot_start(&mut self, _shot_id: u64, _seed: u64) -> Result<()> {
        OPERATION_QUEUE.lock().unwrap().clear();
        self.measurements.clear();
        self.current_result_id = 0;
        Ok(())
    }

    fn shot_end(&mut self) -> Result<()> {
        Ok(())
    }

    fn global_barrier(&mut self, _sleep_ns: u64) -> Result<()> {
        Ok(())
    }

    fn local_barrier(&mut self, _qubits: &[u64], _sleep_ns: u64) -> Result<()> {
        Ok(())
    }

    fn qalloc(&mut self) -> Result<u64> {
        Ok(self.measurements.len() as u64)
    }

    fn qfree(&mut self, _qubit_id: u64) -> Result<()> {
        Ok(())
    }

    fn rxy_gate(&mut self, qubit_id: u64, theta: f64, phi: f64) -> Result<()> {
        OPERATION_QUEUE.lock().unwrap().push_back(Operation::RXYGate { qubit_id, theta, phi });
        Ok(())
    }

    fn rzz_gate(&mut self, qubit_id_1: u64, qubit_id_2: u64, theta: f64) -> Result<()> {
        OPERATION_QUEUE.lock().unwrap().push_back(Operation::RZZGate { qubit_id_1, qubit_id_2, theta });
        Ok(())
    }

    fn rz_gate(&mut self, qubit_id: u64, theta: f64) -> Result<()> {
        OPERATION_QUEUE.lock().unwrap().push_back(Operation::RZGate { qubit_id, theta });
        Ok(())
    }

    fn measure(&mut self, qubit_id: u64) -> Result<u64> {
        let result_id = self.current_result_id;
        self.current_result_id += 1;
        self.measurements.resize(result_id as usize + 1, false);

        OPERATION_QUEUE.lock().unwrap().push_back(Operation::Measure { qubit_id, result_id });
        Ok(result_id)
    }

    fn reset(&mut self, qubit_id: u64) -> Result<()> {
        OPERATION_QUEUE.lock().unwrap().push_back(Operation::Reset { qubit_id });
        Ok(())
    }

    fn force_result(&mut self, _result_id: u64) -> Result<()> {
        Ok(())
    }

    fn get_result(&mut self, result_id: u64) -> Result<Option<bool>> {
        if result_id >= self.measurements.len() as u64 {
            return Ok(None);
        }
        Ok(Some(self.measurements[result_id as usize]))
    }

    fn set_result(&mut self, result_id: u64, result: bool) -> Result<()> {
        if result_id >= self.measurements.len() as u64 {
            self.measurements.resize(result_id as usize + 1, false);
        }
        self.measurements[result_id as usize] = result;
        Ok(())
    }

    fn increment_future_refcount(&mut self, _future_ref: u64) -> Result<()> {
        Ok(())
    }

    fn decrement_future_refcount(&mut self, _future_ref: u64) -> Result<()> {
        Ok(())
    }

    fn get_metric(&mut self, _nth_metric: u8) -> Result<Option<(String, MetricValue)>> {
        Ok(None)
    }
}"#;

const FACTORY_AND_EXPORT: &str = r#"#[derive(Default)]
struct SimulatorRuntimeFactory;

impl RuntimeInterfaceFactory for SimulatorRuntimeFactory {
    type Interface = SimulatorRuntimePlugin;

    fn init(
        self: std::sync::Arc<Self>,
        _n_qubits: u64,
        start: selene_core::time::Instant,
        _args: &[impl AsRef<str>],
    ) -> Result<Box<Self::Interface>> {
        Ok(Box::new(SimulatorRuntimePlugin::new(start)))
    }
}

export_runtime_plugin!(crate::SimulatorRuntimeFactory);"#;
