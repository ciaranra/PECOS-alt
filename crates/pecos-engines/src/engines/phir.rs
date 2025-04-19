use crate::byte_message::{ByteMessage, builder::ByteMessageBuilder};
use crate::core::shot_results::ShotResult;
use crate::engines::{ControlEngine, Engine, EngineStage, classical::ClassicalEngine};
use crate::errors::QueueError;
use log::debug;
use serde::Deserialize;
use std::any::Any;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
struct PHIRProgram {
    format: String,
    version: String,
    metadata: HashMap<String, String>,
    ops: Vec<Operation>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
enum Operation {
    VariableDefinition {
        data: String,
        data_type: String,
        variable: String,
        size: usize,
    },
    QuantumOp {
        qop: String,
        #[serde(default)]
        angles: Option<(Vec<f64>, String)>,
        args: Vec<(String, usize)>,
    },
    ClassicalOp {
        cop: String,
        args: Vec<(String, usize)>,
        returns: Vec<(String, usize)>,
    },
}

#[derive(Debug)]
pub struct PHIREngine {
    program: Option<PHIRProgram>,
    current_op: usize,
    measurement_results: HashMap<String, u32>,
    quantum_variables: HashMap<String, usize>,
    classical_variables: HashMap<String, (String, usize)>,
    message_builder: ByteMessageBuilder,
}

impl PHIREngine {
    /// Creates a new instance of `PHIREngine` by loading a PHIR program JSON file.
    ///
    /// # Parameters
    /// - `path`: A reference to the path of the PHIR program JSON file to load.
    ///
    /// # Returns
    /// - `Ok(Self)`: If the PHIR program file is successfully loaded and validated.
    /// - `Err(Box<dyn impl PHIREngine {std::error::Error>)`: If any errors occur during file reading,
    ///   parsing, or if the format/version is not compatible.
    ///
    /// # Errors
    /// - Returns an error if the file cannot be read.
    /// - Returns an error if the JSON parsing fails.
    /// - Returns an error if the format is not "PHIR/JSON".
    /// - Returns an error if the version is not "0.1.0".
    ///
    /// # Examples
    /// ```rust
    /// use pecos_engines::engines::phir::PHIREngine;
    ///
    /// let engine = PHIREngine::new("path_to_program.json");
    /// match engine {
    ///     Ok(engine) => println!("PHIREngine loaded successfully!"),
    ///     Err(e) => eprintln!("Error loading PHIREngine: {}", e),
    /// }
    /// ```
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let program: PHIRProgram = serde_json::from_str(&content)?;

        if program.format != "PHIR/JSON" {
            return Err("Invalid format: expected PHIR/JSON".into());
        }

        if program.version != "0.1.0" {
            return Err(format!("Unsupported PHIR version: {}", program.version).into());
        }

        log::debug!("Loading PHIR program with metadata: {:?}", program.metadata);

        Ok(Self {
            program: Some(program),
            current_op: 0,
            measurement_results: HashMap::new(),
            quantum_variables: HashMap::new(),
            classical_variables: HashMap::new(),
            message_builder: ByteMessageBuilder::new(),
        })
    }

    fn reset_state(&mut self) {
        debug!(
            "INTERNAL RESET: PHIREngine reset before current_op={}",
            self.current_op
        );
        self.current_op = 0;
        debug!(
            "INTERNAL RESET: PHIREngine reset after current_op={}",
            self.current_op
        );
        self.measurement_results.clear();
        // Reset the message builder to reuse allocated memory
        self.message_builder.reset();
    }

    // Create an empty engine without any program
    fn empty() -> Self {
        Self {
            program: None,
            current_op: 0,
            measurement_results: HashMap::new(),
            quantum_variables: HashMap::new(),
            classical_variables: HashMap::new(),
            message_builder: ByteMessageBuilder::new(),
        }
    }

    fn handle_variable_definition(
        &mut self,
        data: &str,
        data_type: &str,
        variable: &str,
        size: usize,
    ) {
        match data {
            "qvar_define" if data_type == "qubits" => {
                self.quantum_variables.insert(variable.to_string(), size);
                log::debug!("Defined quantum variable {} of size {}", variable, size);
            }
            "cvar_define" => {
                self.classical_variables
                    .insert(variable.to_string(), (data_type.to_string(), size));
                log::debug!(
                    "Defined classical variable {} of type {} and size {}",
                    variable,
                    data_type,
                    size
                );
            }
            _ => log::warn!(
                "Unknown variable definition: {} {} {}",
                data,
                data_type,
                variable
            ),
        }
    }

    fn validate_variable_access(&self, var: &str, idx: usize) -> Result<(), QueueError> {
        // Check quantum variables
        if let Some(&size) = self.quantum_variables.get(var) {
            if idx >= size {
                return Err(QueueError::OperationError(format!(
                    "Index {idx} out of bounds for quantum variable {var} of size {size}"
                )));
            }
            return Ok(());
        }

        // Check classical variables
        if let Some((_, size)) = self.classical_variables.get(var) {
            if idx >= *size {
                return Err(QueueError::OperationError(format!(
                    "Index {idx} out of bounds for classical variable {var} of size {size}"
                )));
            }
            return Ok(());
        }

        Err(QueueError::OperationError(format!(
            "Undefined variable: {var}"
        )))
    }

    fn handle_classical_op(
        &mut self,
        cop: &str,
        args: &[(String, usize)],
        returns: &[(String, usize)],
    ) -> Result<bool, QueueError> {
        // Validate all variable accesses
        for (var, idx) in args.iter().chain(returns) {
            self.validate_variable_access(var, *idx)?;
        }

        if cop == "Result" {
            let meas_var = &args[0].0;
            let meas_idx = args[0].1;
            let return_var = &returns[0].0;
            let return_idx = returns[0].1;

            log::debug!(
                "Will store measurement {}[{}] in return location {}[{}]",
                meas_var,
                meas_idx,
                return_var,
                return_idx
            );

            // Return true if this is the last Result operation in a sequence
            // We can check this by looking at the next operation
            if let Some(prog) = &self.program {
                let next_op = prog.ops.get(self.current_op + 1);
                match next_op {
                    Some(Operation::ClassicalOp {
                        cop: result_cop, ..
                    }) if result_cop == "Result" => {
                        // More Result operations coming, keep accumulating
                        Ok(false)
                    }
                    _ => {
                        // No more Result operations, flush the batch
                        Ok(true)
                    }
                }
            } else {
                Ok(true)
            }
        } else {
            Ok(false)
        }
    }

    #[allow(clippy::too_many_lines)]
    #[allow(clippy::items_after_statements)]
    fn generate_commands(&mut self) -> Result<ByteMessage, QueueError> {
        // Define a maximum batch size for better performance
        // This helps avoid creating excessively large messages
        const MAX_BATCH_SIZE: usize = 100;

        debug!(
            "Generating commands - thread {:?}, current_op: {}",
            std::thread::current().id(),
            self.current_op
        );

        // Get program reference and clone ops to avoid borrow issues
        let prog = self
            .program
            .as_ref()
            .ok_or_else(|| QueueError::OperationError("No program loaded".into()))?;
        let ops = prog.ops.clone();

        // If we've processed all ops, return empty batch to signal completion
        if self.current_op >= ops.len() {
            debug!("End of program reached, sending flush");
            return Ok(ByteMessage::create_flush());
        }

        // Reset and configure the reusable message builder for quantum operations
        self.message_builder.reset();
        let _ = self.message_builder.for_quantum_operations();
        let mut operation_count = 0;

        while self.current_op < ops.len() && operation_count < MAX_BATCH_SIZE {
            match &ops[self.current_op] {
                Operation::VariableDefinition {
                    data,
                    data_type,
                    variable,
                    size,
                } => {
                    debug!(
                        "Processing variable definition: {} {} {}",
                        data, data_type, variable
                    );
                    self.handle_variable_definition(data, data_type, variable, *size);
                }
                Operation::QuantumOp { qop, angles, args } => {
                    debug!("Processing quantum operation: {}", qop);

                    // Clone the operation parameters to avoid borrow issues
                    let qop_str = qop.clone();
                    let args_clone = args.clone();
                    let angles_clone = angles.clone();

                    // Process the quantum operation
                    // This avoids borrowing self and self.message_builder at the same time
                    match self.process_quantum_op(&qop_str, angles_clone.as_ref(), &args_clone) {
                        Ok((gate_type, qubit_args, angle_args)) => {
                            // Now add the gate to the builder based on the processed parameters
                            match gate_type.as_str() {
                                "RZ" => {
                                    self.message_builder.add_rz(angle_args[0], &[qubit_args[0]]);
                                }
                                "R1XY" => {
                                    self.message_builder.add_r1xy(
                                        angle_args[0],
                                        angle_args[1],
                                        &[qubit_args[0]],
                                    );
                                }
                                "SZZ" => {
                                    self.message_builder
                                        .add_szz(&[qubit_args[0]], &[qubit_args[1]]);
                                }
                                "CX" => {
                                    self.message_builder
                                        .add_cx(&[qubit_args[0]], &[qubit_args[1]]);
                                }
                                "H" => {
                                    self.message_builder.add_h(&[qubit_args[0]]);
                                }
                                "X" => {
                                    self.message_builder.add_x(&[qubit_args[0]]);
                                }
                                "Y" => {
                                    self.message_builder.add_y(&[qubit_args[0]]);
                                }
                                "Z" => {
                                    self.message_builder.add_z(&[qubit_args[0]]);
                                }
                                "Measure" => {
                                    self.message_builder
                                        .add_measurements(&[qubit_args[0]], &[qubit_args[0]]);
                                }
                                _ => {
                                    return Err(QueueError::OperationError(format!(
                                        "Unsupported quantum operation: {gate_type}"
                                    )));
                                }
                            }
                            operation_count += 1;
                            debug!("Added quantum operation to builder");
                        }
                        Err(e) => return Err(e),
                    }
                }
                Operation::ClassicalOp { cop, args, returns } => {
                    debug!("Processing classical operation: {}", cop);
                    if self.handle_classical_op(cop, args, returns)? {
                        debug!("Finishing batch due to classical operation completion");
                        self.current_op += 1;

                        // Build and return the message
                        if operation_count > 0 {
                            debug!("Returning batch with {} operations", operation_count);
                            return Ok(self.message_builder.build());
                        }

                        // Create an empty message if no operations were added
                        debug!("Returning empty batch after classical operation");
                        return Ok(ByteMessage::builder().build());
                    }
                }
            }
            self.current_op += 1;

            // If we've reached the maximum batch size, break out of the loop
            // This ensures we don't create excessively large messages
            if operation_count >= MAX_BATCH_SIZE {
                debug!(
                    "Reached maximum batch size ({}), returning current batch",
                    MAX_BATCH_SIZE
                );
                break;
            }
        }

        debug!(
            "PHIR engine generated {} operations for shot",
            operation_count
        );

        // Build and return the message
        Ok(self.message_builder.build())
    }

    /// Process a quantum operation and return the gate type, qubit arguments, and angle arguments
    fn process_quantum_op(
        &self,
        qop: &str,
        angles: Option<&(Vec<f64>, String)>,
        args: &[(String, usize)],
    ) -> Result<(String, Vec<usize>, Vec<f64>), QueueError> {
        // First validate all variables
        for (var, idx) in args {
            self.validate_variable_access(var, *idx)?;
        }

        // Validate that we have at least one qubit argument
        if args.is_empty() {
            return Err(QueueError::OperationError(format!(
                "Operation {qop} requires at least one qubit argument"
            )));
        }

        // Extract qubit arguments
        let mut qubit_args = Vec::new();
        for (_, idx) in args {
            qubit_args.push(*idx);
        }

        // Process based on gate type
        match qop {
            // Single-qubit rotation gates
            "RZ" => {
                let theta = angles
                    .as_ref()
                    .map(|(angles, _)| angles[0])
                    .ok_or_else(|| {
                        QueueError::OperationError(format!("Missing angle for {qop} gate"))
                    })?;
                Ok((qop.to_string(), qubit_args, vec![theta]))
            }
            "R1XY" => {
                if angles.as_ref().map_or(0, |(angles, _)| angles.len()) < 2 {
                    return Err(QueueError::OperationError(format!(
                        "{qop} gate requires two angles (phi, theta)"
                    )));
                }
                let (phi, theta) = angles
                    .as_ref()
                    .map(|(angles, _)| (angles[0], angles[1]))
                    .ok_or_else(|| {
                        QueueError::OperationError(format!("Missing angles for {qop} gate"))
                    })?;
                Ok((qop.to_string(), qubit_args, vec![phi, theta]))
            }

            // Two-qubit gates
            "SZZ" | "ZZ" => {
                if args.len() < 2 {
                    return Err(QueueError::OperationError(format!(
                        "{qop} gate requires two qubits"
                    )));
                }
                Ok(("SZZ".to_string(), qubit_args, vec![]))
            }
            "CX" | "CNOT" => {
                if args.len() < 2 {
                    return Err(QueueError::OperationError(format!(
                        "{qop} gate requires control and target qubits"
                    )));
                }
                Ok(("CX".to_string(), qubit_args, vec![]))
            }

            // Single-qubit Clifford gates
            // Single-qubit Clifford gates and Measurement
            "H" | "X" | "Y" | "Z" | "Measure" => Ok((qop.to_string(), qubit_args, vec![])),

            _ => Err(QueueError::OperationError(format!(
                "Unsupported quantum operation: {qop}"
            ))),
        }
    }
}

impl Default for PHIREngine {
    fn default() -> Self {
        Self::empty()
    }
}

impl ControlEngine for PHIREngine {
    type Input = ();
    type Output = ShotResult;
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;

    fn start(&mut self, _input: ()) -> Result<EngineStage<ByteMessage, ShotResult>, QueueError> {
        debug!(
            "PHIR: start() called with current_op={}, beginning new shot",
            self.current_op
        );
        self.current_op = 0; // Force reset here too
        self.measurement_results.clear();

        let commands = self.generate_commands()?;
        if commands.is_empty().unwrap_or(false) {
            debug!("PHIR: start() - No commands to process, returning results immediately");
            Ok(EngineStage::Complete(self.get_results()?))
        } else {
            debug!("PHIR: start() - Returning commands for processing");
            Ok(EngineStage::NeedsProcessing(commands))
        }
    }

    fn continue_processing(
        &mut self,
        measurements: ByteMessage,
    ) -> Result<EngineStage<ByteMessage, ShotResult>, QueueError> {
        // Handle received measurements
        self.handle_measurements(measurements)?;

        // Get next batch of commands if any
        let commands = self.generate_commands()?;
        if commands.is_empty().unwrap_or(false) {
            Ok(EngineStage::Complete(self.get_results()?))
        } else {
            Ok(EngineStage::NeedsProcessing(commands))
        }
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        debug!("PHIREngine::reset() implementation for ControlEngine being called!");
        self.reset_state();
        Ok(())
    }
}

impl ClassicalEngine for PHIREngine {
    #[allow(clippy::too_many_lines)]
    fn generate_commands(&mut self) -> Result<ByteMessage, QueueError> {
        // Define a maximum batch size for better performance
        // This helps avoid creating excessively large messages
        const MAX_BATCH_SIZE: usize = 100;

        debug!(
            "Generating commands - thread {:?}, current_op: {}",
            std::thread::current().id(),
            self.current_op
        );

        // Get program reference and clone ops to avoid borrow issues
        let prog = self
            .program
            .as_ref()
            .ok_or_else(|| QueueError::OperationError("No program loaded".into()))?;
        let ops = prog.ops.clone();

        // If we've processed all ops, return empty batch to signal completion
        if self.current_op >= ops.len() {
            debug!("End of program reached, sending flush");
            return Ok(ByteMessage::create_flush());
        }

        // Reset and configure the reusable message builder for quantum operations
        self.message_builder.reset();
        let _ = self.message_builder.for_quantum_operations();
        let mut operation_count = 0;

        while self.current_op < ops.len() && operation_count < MAX_BATCH_SIZE {
            match &ops[self.current_op] {
                Operation::VariableDefinition {
                    data,
                    data_type,
                    variable,
                    size,
                } => {
                    debug!(
                        "Processing variable definition: {} {} {}",
                        data, data_type, variable
                    );
                    self.handle_variable_definition(data, data_type, variable, *size);
                }
                Operation::QuantumOp { qop, angles, args } => {
                    debug!("Processing quantum operation: {}", qop);

                    // Clone the operation parameters to avoid borrow issues
                    let qop_str = qop.clone();
                    let args_clone = args.clone();
                    let angles_clone = angles.clone();

                    // Process the quantum operation
                    // This avoids borrowing self and self.message_builder at the same time
                    match self.process_quantum_op(&qop_str, angles_clone.as_ref(), &args_clone) {
                        Ok((gate_type, qubit_args, angle_args)) => {
                            // Now add the gate to the builder based on the processed parameters
                            match gate_type.as_str() {
                                "RZ" => {
                                    self.message_builder.add_rz(angle_args[0], &[qubit_args[0]]);
                                }
                                "R1XY" => {
                                    self.message_builder.add_r1xy(
                                        angle_args[0],
                                        angle_args[1],
                                        &[qubit_args[0]],
                                    );
                                }
                                "SZZ" => {
                                    self.message_builder
                                        .add_szz(&[qubit_args[0]], &[qubit_args[1]]);
                                }
                                "CX" => {
                                    self.message_builder
                                        .add_cx(&[qubit_args[0]], &[qubit_args[1]]);
                                }
                                "H" => {
                                    self.message_builder.add_h(&[qubit_args[0]]);
                                }
                                "X" => {
                                    self.message_builder.add_x(&[qubit_args[0]]);
                                }
                                "Y" => {
                                    self.message_builder.add_y(&[qubit_args[0]]);
                                }
                                "Z" => {
                                    self.message_builder.add_z(&[qubit_args[0]]);
                                }
                                "Measure" => {
                                    self.message_builder
                                        .add_measurements(&[qubit_args[0]], &[qubit_args[0]]);
                                }
                                _ => {
                                    return Err(QueueError::OperationError(format!(
                                        "Unsupported quantum operation: {gate_type}"
                                    )));
                                }
                            }
                            operation_count += 1;
                            debug!("Added quantum operation to builder");
                        }
                        Err(e) => return Err(e),
                    }
                }
                Operation::ClassicalOp { cop, args, returns } => {
                    debug!("Processing classical operation: {}", cop);
                    if self.handle_classical_op(cop, args, returns)? {
                        debug!("Finishing batch due to classical operation completion");
                        self.current_op += 1;

                        // Build and return the message
                        if operation_count > 0 {
                            debug!("Returning batch with {} operations", operation_count);
                            return Ok(self.message_builder.build());
                        }

                        // Create an empty message if no operations were added
                        debug!("Returning empty batch after classical operation");
                        return Ok(ByteMessage::builder().build());
                    }
                }
            }
            self.current_op += 1;

            // If we've reached the maximum batch size, break out of the loop
            // This ensures we don't create excessively large messages
            if operation_count >= MAX_BATCH_SIZE {
                debug!(
                    "Reached maximum batch size ({}), returning current batch",
                    MAX_BATCH_SIZE
                );
                break;
            }
        }

        debug!(
            "PHIR engine generated {} operations for shot",
            operation_count
        );

        // Build and return the message
        Ok(self.message_builder.build())
    }

    fn num_qubits(&self) -> usize {
        // First check if quantum_variables is already populated
        let sum: usize = self.quantum_variables.values().sum();
        if sum > 0 {
            return sum;
        }

        // If quantum_variables is empty, directly scan the program ops
        if let Some(program) = &self.program {
            let mut total = 0;
            for op in &program.ops {
                if let Operation::VariableDefinition {
                    data,
                    data_type,
                    variable: _,
                    size,
                } = op
                {
                    if data == "qvar_define" && data_type == "qubits" {
                        total += size;
                    }
                }
            }
            return total;
        }

        0 // If no program is loaded, return 0
    }

    fn handle_measurements(&mut self, message: ByteMessage) -> Result<(), QueueError> {
        // Parse measurements using ByteMessage helper
        let measurements = message.parse_measurements()?;

        for (result_id, outcome) in measurements {
            debug!(
                "PHIR: Received measurement {}, result_id={}",
                outcome, result_id
            );

            // Store the measurement
            self.measurement_results
                .insert(format!("measurement_{result_id}"), outcome);
        }

        Ok(())
    }

    fn get_results(&self) -> Result<ShotResult, QueueError> {
        let mut results = ShotResult::default();

        debug!(
            "PHIR: Getting results from {} measurements",
            self.measurement_results.len()
        );

        // First add all individual measurements to the results
        for (key, &value) in &self.measurement_results {
            results.measurements.insert(key.clone(), value);
        }

        // Build a combined result string from measurement_0 and measurement_1
        let mut result_digits = String::new();

        // Look for measurement_0 and measurement_1 in order
        if let Some(&m0) = self.measurement_results.get("measurement_0") {
            result_digits.push_str(&m0.to_string());
        }
        if let Some(&m1) = self.measurement_results.get("measurement_1") {
            result_digits.push_str(&m1.to_string());
        }

        debug!("PHIR: Combined measurement result: {}", result_digits);

        // Store combined result string as u32 if possible
        if !result_digits.is_empty() {
            if let Ok(result_value) = result_digits.parse::<u32>() {
                results
                    .measurements
                    .insert("result".to_string(), result_value);
            } else {
                // If parsing fails, store a default value and log warning
                debug!("PHIR: Could not parse '{}' as u32", result_digits);
                results.measurements.insert("result".to_string(), 0);
            }
        }

        Ok(results)
    }

    fn compile(&self) -> Result<(), Box<dyn std::error::Error>> {
        // No compilation needed for PHIR/JSON
        Ok(())
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        debug!("PHIREngine::reset() implementation for ClassicalEngine being called!");
        self.reset_state();
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Clone for PHIREngine {
    fn clone(&self) -> Self {
        // Create a new instance with the same program
        match &self.program {
            Some(program) => Self {
                program: Some(program.clone()),
                current_op: 0, // Reset state in the clone
                measurement_results: HashMap::new(),
                quantum_variables: self.quantum_variables.clone(),
                classical_variables: self.classical_variables.clone(),
                message_builder: ByteMessageBuilder::new(),
            },
            None => Self::empty(),
        }
    }
}

impl Engine for PHIREngine {
    type Input = ();
    type Output = ShotResult;

    fn process(&mut self, _input: Self::Input) -> Result<Self::Output, QueueError> {
        // Process operations until we need more input or we're done
        let mut stage = self.start(())?;

        // If we're already done, return the result
        if let EngineStage::Complete(result) = stage {
            return Ok(result);
        }

        // Otherwise, we need to process more (just return an empty measurement result)
        if let EngineStage::NeedsProcessing(_) = stage {
            // Create an empty message to simulate processing
            let empty_message = ByteMessage::builder().build();
            stage = self.continue_processing(empty_message)?;

            if let EngineStage::Complete(result) = stage {
                return Ok(result);
            }
        }

        // If we get here, something went wrong
        Err(QueueError::OperationError(
            "Failed to complete processing".into(),
        ))
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        // Call our internal reset method
        self.reset_state();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_phir_engine_basic() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let program_path = dir.path().join("test.json");

        // Create a test program
        let program = r#"{
    "format": "PHIR/JSON",
    "version": "0.1.0",
    "metadata": {"test": "true"},
    "ops": [
        {
            "data": "qvar_define",
            "data_type": "qubits",
            "variable": "q",
            "size": 2
        },
        {
            "data": "cvar_define",
            "data_type": "i64",
            "variable": "m",
            "size": 2
        },
        {
            "data": "cvar_define",
            "data_type": "i64",
            "variable": "result",
            "size": 2
        },
        {
            "qop": "R1XY",
            "angles": [[0.1, 0.2], "rad"],
            "args": [["q", 0]]
        },
        {
            "qop": "Measure",
            "args": [["q", 0]],
            "returns": [["m", 0]]
        },
        {"cop": "Result", "args": [["m", 0]], "returns": [["result", 0]]}
    ]
}"#;

        let mut file = File::create(&program_path)?;
        file.write_all(program.as_bytes())?;

        let mut engine = PHIREngine::new(&program_path)?;

        // Generate commands and verify they're correctly generated
        let command_message = engine.generate_commands()?;

        // Parse the message back to confirm it has the correct operations
        let parsed_commands = command_message.parse_quantum_operations()?;
        assert_eq!(parsed_commands.len(), 2);

        // Create a measurement message and test handling
        // result_id=0, outcome=1
        let message = ByteMessage::builder()
            .add_measurement_results(&[1], &[0])
            .build();

        engine.handle_measurements(message)?;

        // Verify results
        let results = engine.get_results()?;
        assert_eq!(results.measurements.len(), 2);
        assert_eq!(results.measurements["measurement_0"], 1);

        Ok(())
    }
}
