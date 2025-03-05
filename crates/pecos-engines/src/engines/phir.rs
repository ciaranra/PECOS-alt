use super::{ClassicalEngine, ControlEngine, EngineStage};
use crate::channels::byte_message::ByteMessage;
use crate::errors::QueueError;
use log::debug;
use pecos_core::types::{GateType, QuantumCommand, ShotResult};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use crate::channels::byte::builder::MessageBuilder;

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
        })
    }

    fn reset_internal_state(&mut self) {
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
    }

    // Create an empty engine without any program
    fn empty() -> Self {
        Self {
            program: None,
            current_op: 0,
            measurement_results: HashMap::new(),
            quantum_variables: HashMap::new(),
            classical_variables: HashMap::new(),
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

    fn handle_quantum_op(
        &mut self,
        qop: &str,
        angles: Option<&(Vec<f64>, String)>,
        args: &[(String, usize)],
    ) -> Result<QuantumCommand, QueueError> {
        // First validate all variables
        for (var, idx) in args {
            self.validate_variable_access(var, *idx)?;
        }

        // Now create command based on gathered data
        Ok(match qop {
            "RZ" => {
                let theta = angles
                    .as_ref()
                    .map(|(angles, _)| angles[0])
                    .ok_or_else(|| QueueError::OperationError("Missing angle for RZ".into()))?;
                QuantumCommand {
                    gate: GateType::RZ { theta },
                    qubits: vec![args[0].1],
                }
            }
            "R1XY" => {
                let (phi, theta) = angles
                    .as_ref()
                    .map(|(angles, _)| (angles[0], angles[1]))
                    .ok_or_else(|| QueueError::OperationError("Missing angles for R1XY".into()))?;
                QuantumCommand {
                    gate: GateType::R1XY { phi, theta },
                    qubits: vec![args[0].1],
                }
            }
            "SZZ" => QuantumCommand {
                gate: GateType::SZZ,
                qubits: vec![args[0].1, args[1].1],
            },
            "H" => QuantumCommand {
                gate: GateType::H,
                qubits: vec![args[0].1],
            },
            "CX" => {
                if args.len() != 2 {
                    return Err(QueueError::OperationError(
                        "CX gate requires control and target qubits".into(),
                    ));
                }
                QuantumCommand {
                    gate: GateType::CX,
                    qubits: vec![args[0].1, args[1].1],
                }
            }
            "Measure" => {
                let result_id = args[0].1;
                QuantumCommand {
                    gate: GateType::Measure { result_id },
                    qubits: vec![args[0].1],
                }
            }
            _ => {
                return Err(QueueError::OperationError(format!(
                    "Unknown quantum operation: {qop}"
                )));
            }
        })
    }

    #[allow(clippy::similar_names)]
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
                    Some(Operation::ClassicalOp { cop: next_cop, .. }) if next_cop == "Result" => {
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
            "PHIREngine start() called with current_op={}",
            self.current_op
        );
        self.current_op = 0; // Force reset here too
        self.measurement_results.clear();

        let commands = self.generate_commands()?;
        if commands.is_empty().unwrap_or(false) {
            Ok(EngineStage::Complete(self.get_results()?))
        } else {
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
        self.reset_internal_state();
        Ok(())
    }
}

impl ClassicalEngine for PHIREngine {
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

    fn generate_commands(&mut self) -> Result<ByteMessage, QueueError> {
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
            return Ok(MessageBuilder::create_flush_message(true));
        }

        let mut commands = Vec::new();

        while self.current_op < ops.len() {
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
                    let cmd = self.handle_quantum_op(qop, angles.as_ref(), args)?;
                    debug!("Generated quantum command: {:?}", cmd);
                    commands.push(cmd);
                }
                Operation::ClassicalOp { cop, args, returns } => {
                    debug!("Processing classical operation: {}", cop);
                    if self.handle_classical_op(cop, args, returns)? {
                        debug!("Finishing batch due to classical operation completion");
                        self.current_op += 1;

                        return Ok(MessageBuilder::create_quantum_message(&commands));
                    }
                }
            }
            self.current_op += 1;
        }

        debug!("PHIR engine generated {} commands for shot", commands.len());

        Ok(MessageBuilder::create_quantum_message(&commands))
    }

    fn handle_measurements(&mut self, message: ByteMessage) -> Result<(), QueueError> {
        // Parse measurements using ByteMessage helper
        let measurements = message.parse_measurements()?;

        for measurement in measurements {
            // Extract result_id and outcome directly
            let result_id = (measurement >> 16) as usize;
            let outcome = measurement & 0xFFFF;

            debug!(
                "PHIR: Received measurement {} (encoded as {}), result_id={}",
                outcome, measurement, result_id
            );

            // Store the measurement
            self.measurement_results
                .insert(format!("measurement_{result_id}"), outcome);

            debug!(
                "PHIR: Stored measurement {} at result_id {}",
                outcome, result_id
            );
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
        self.current_op = 0;
        debug!("Reset current_op to {}", self.current_op);
        self.measurement_results.clear();
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn ClassicalEngine> {
        Box::new(self.clone())
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
            },
            None => Self::empty(),
        }
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
        let measurement = 1u32; // result_id=0, outcome=1
        let message = ByteMessage::create_measurements(&[measurement])?;
        engine.handle_measurements(message)?;

        // Verify results
        let results = engine.get_results()?;
        assert_eq!(results.measurements.len(), 2);
        assert_eq!(results.measurements["measurement_0"], 1);

        Ok(())
    }
}
