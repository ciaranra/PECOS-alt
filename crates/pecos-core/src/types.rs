// PECOS/crates/pecos-engines/src/types.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GateType {
    X,
    Y,
    Z,
    RZ { theta: f64 },
    R1XY { phi: f64, theta: f64 },
    SZZ,
    H,
    CX,
    Measure { result_id: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumCommand {
    pub gate: GateType,
    pub qubits: Vec<usize>,
}

impl QuantumCommand {
    /// Parses quantum circuit commands from string representation.
    ///
    /// # Format
    /// Commands must follow these formats:
    /// - RZ theta qubit
    /// - R1XY phi theta qubit
    /// - SZZ qubit1 qubit2
    /// - M qubit `result_id`
    ///
    /// All numeric parameters should be valid floating point numbers.
    /// Qubits and `result_id` should be valid integers.
    ///
    /// # Examples
    /// ```
    /// use pecos_core::types::QuantumCommand;
    /// let cmd = QuantumCommand::parse_from_str("RZ 0.5 1").unwrap();
    /// let cmd = QuantumCommand::parse_from_str("R1XY 0.1 0.2 0").unwrap();
    /// let cmd = QuantumCommand::parse_from_str("SZZ 0 1").unwrap();
    /// let cmd = QuantumCommand::parse_from_str("M 0 42").unwrap();
    /// ```
    ///
    /// # Errors
    /// Returns error strings for:
    /// - Wrong number of parameters for command type
    /// - Invalid numeric values for angles/ids
    /// - Unknown command type
    /// - Empty command string
    #[allow(clippy::too_many_lines)]
    pub fn parse_from_str(cmd_str: &str) -> Result<Self, String> {
        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
        match parts.first() {
            Some(&"RZ") => {
                if parts.len() != 3 {
                    return Err("Invalid RZ format".into());
                }
                Ok(Self {
                    gate: GateType::RZ {
                        theta: parts[1]
                            .parse()
                            .map_err(|e| format!("Invalid theta: {e}"))?,
                    },
                    qubits: vec![
                        parts[2]
                            .parse()
                            .map_err(|e| format!("Invalid qubit: {e}"))?,
                    ],
                })
            }
            Some(&"R1XY") => {
                if parts.len() != 4 {
                    return Err("Invalid R1XY format".into());
                }
                Ok(Self {
                    gate: GateType::R1XY {
                        phi: parts[1].parse().map_err(|e| format!("Invalid phi: {e}"))?,
                        theta: parts[2]
                            .parse()
                            .map_err(|e| format!("Invalid theta: {e}"))?,
                    },
                    qubits: vec![
                        parts[3]
                            .parse()
                            .map_err(|e| format!("Invalid qubit: {e}"))?,
                    ],
                })
            }
            Some(&"SZZ") => {
                if parts.len() != 3 {
                    return Err("Invalid SZZ format".into());
                }
                Ok(Self {
                    gate: GateType::SZZ,
                    qubits: vec![
                        parts[1]
                            .parse()
                            .map_err(|e| format!("Invalid qubit1: {e}"))?,
                        parts[2]
                            .parse()
                            .map_err(|e| format!("Invalid qubit2: {e}"))?,
                    ],
                })
            }
            Some(&"X") => {
                if parts.len() != 2 {
                    return Err("Invalid X format".into());
                }
                Ok(Self {
                    gate: GateType::X,
                    qubits: vec![
                        parts[1]
                            .parse()
                            .map_err(|e| format!("Invalid qubit: {e}"))?,
                    ],
                })
            }
            Some(&"Y") => {
                if parts.len() != 2 {
                    return Err("Invalid Y format".into());
                }
                Ok(Self {
                    gate: GateType::Y,
                    qubits: vec![
                        parts[1]
                            .parse()
                            .map_err(|e| format!("Invalid qubit: {e}"))?,
                    ],
                })
            }
            Some(&"Z") => {
                if parts.len() != 2 {
                    return Err("Invalid Z format".into());
                }
                Ok(Self {
                    gate: GateType::Z,
                    qubits: vec![
                        parts[1]
                            .parse()
                            .map_err(|e| format!("Invalid qubit: {e}"))?,
                    ],
                })
            }
            Some(&"H") => {
                if parts.len() != 2 {
                    return Err("Invalid H format".into());
                }
                Ok(Self {
                    gate: GateType::H,
                    qubits: vec![
                        parts[1]
                            .parse()
                            .map_err(|e| format!("Invalid qubit: {e}"))?,
                    ],
                })
            }
            Some(&"CX") => {
                if parts.len() != 3 {
                    return Err("Invalid CX format".into());
                }
                Ok(Self {
                    gate: GateType::CX,
                    qubits: vec![
                        parts[1]
                            .parse()
                            .map_err(|e| format!("Invalid control qubit: {e}"))?,
                        parts[2]
                            .parse()
                            .map_err(|e| format!("Invalid target qubit: {e}"))?,
                    ],
                })
            }
            Some(&"M") => {
                if parts.len() != 3 {
                    return Err("Invalid M format".into());
                }
                Ok(Self {
                    gate: GateType::Measure {
                        result_id: parts[2]
                            .parse()
                            .map_err(|e| format!("Invalid result_id: {e}"))?,
                    },
                    qubits: vec![
                        parts[1]
                            .parse()
                            .map_err(|e| format!("Invalid qubit: {e}"))?,
                    ],
                })
            }
            _ => Err(format!(
                "Unknown command type: {}",
                parts.first().unwrap_or(&"<empty>")
            )),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ShotResult {
    pub measurements: HashMap<String, u32>,
}

#[derive(Debug, Clone)]
pub struct ShotResults {
    pub shots: Vec<HashMap<String, String>>,
}

impl Default for ShotResults {
    fn default() -> Self {
        Self::new()
    }
}

impl ShotResults {
    #[must_use]
    pub fn new() -> Self {
        Self { shots: Vec::new() }
    }

    #[must_use]
    pub fn from_measurements(results: &[ShotResult]) -> Self {
        let mut shots = Vec::new();

        for shot in results {
            let mut processed_results: HashMap<String, String> = HashMap::new();
            let mut measurement_values = Vec::new();

            let mut keys: Vec<_> = shot.measurements.keys().collect();
            keys.sort();

            for key in &keys {
                if key.starts_with("measurement_") {
                    if let Some(&value) = shot.measurements.get(*key) {
                        measurement_values.push(value.to_string());
                    }
                } else if let Some(&value) = shot.measurements.get(*key) {
                    processed_results.insert((*key).to_string(), value.to_string());
                }
            }

            if !measurement_values.is_empty() {
                processed_results.insert("result".to_string(), measurement_values.concat());
            }

            shots.push(processed_results);
        }

        Self { shots }
    }

    pub fn print(&self) {
        println!("{self}");
    }
}

impl fmt::Display for ShotResults {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "[")?;

        for (i, shot) in self.shots.iter().enumerate() {
            // Get all keys and sort them for consistent output
            let mut keys: Vec<_> = shot.keys().collect();
            keys.sort();

            write!(f, "  {{")?;
            for (j, key) in keys.iter().enumerate() {
                write!(f, "\"{}\": \"{}\"", key, shot.get(*key).unwrap())?;
                if j < keys.len() - 1 {
                    write!(f, ", ")?;
                }
            }
            if i < self.shots.len() - 1 {
                writeln!(f, "}},")?;
            } else {
                writeln!(f, "}}")?;
            }
        }

        write!(f, "]")
    }
}

#[derive(Debug, Clone)]
pub struct CommandBatch {
    commands: Vec<QuantumCommand>,
    measurement_count: usize,
}

impl Default for CommandBatch {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandBatch {
    #[must_use]
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            measurement_count: 0,
        }
    }

    pub fn add_command(&mut self, cmd: QuantumCommand) {
        if let GateType::Measure { .. } = cmd.gate {
            self.measurement_count += 1;
        }
        self.commands.push(cmd);
    }

    #[must_use]
    pub fn commands(&self) -> &[QuantumCommand] {
        &self.commands
    }

    pub fn commands_mut(&mut self) -> &mut Vec<QuantumCommand> {
        &mut self.commands
    }

    pub fn take_commands(&mut self) -> Vec<QuantumCommand> {
        std::mem::take(&mut self.commands)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn clear(&mut self) {
        self.commands.clear();
        self.measurement_count = 0;
    }

    #[must_use]
    pub fn expected_measurements(&self) -> usize {
        self.measurement_count
    }

    /// Returns an iterator over the commands in the batch.
    pub fn iter(&self) -> std::slice::Iter<'_, QuantumCommand> {
        self.commands.iter()
    }

    /// Returns a mutable iterator over the commands in the batch.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, QuantumCommand> {
        self.commands.iter_mut()
    }
}

impl From<Vec<QuantumCommand>> for CommandBatch {
    fn from(commands: Vec<QuantumCommand>) -> Self {
        let measurement_count = commands
            .iter()
            .filter(|cmd| matches!(cmd.gate, GateType::Measure { .. }))
            .count();
        Self {
            commands,
            measurement_count,
        }
    }
}

impl From<CommandBatch> for Vec<QuantumCommand> {
    fn from(batch: CommandBatch) -> Self {
        batch.commands
    }
}

impl IntoIterator for CommandBatch {
    type Item = QuantumCommand;
    type IntoIter = std::vec::IntoIter<QuantumCommand>;

    fn into_iter(self) -> Self::IntoIter {
        self.commands.into_iter()
    }
}

impl<'a> IntoIterator for &'a CommandBatch {
    type Item = &'a QuantumCommand;
    type IntoIter = std::slice::Iter<'a, QuantumCommand>;

    fn into_iter(self) -> Self::IntoIter {
        self.commands.iter()
    }
}

impl<'a> IntoIterator for &'a mut CommandBatch {
    type Item = &'a mut QuantumCommand;
    type IntoIter = std::slice::IterMut<'a, QuantumCommand>;

    fn into_iter(self) -> Self::IntoIter {
        self.commands.iter_mut()
    }
}
