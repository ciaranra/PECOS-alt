use crate::common::get_thread_id;
use log::debug;
use pecos_core::errors::PecosError;
use pecos_engines::byte_message::{ByteMessage, Gate};
use pecos_engines::core::record_data::RecordData;

/// QIR-specific command type for mixed collections
#[derive(Debug, Clone, PartialEq)]
pub enum QirCommand {
    /// Quantum gate operation
    Gate(Gate),
    /// Record command with structured data
    Record(RecordData),
}

impl QirCommand {
    /// Check if this command is a gate operation
    #[must_use]
    pub fn is_gate(&self) -> bool {
        matches!(self, QirCommand::Gate(_))
    }

    /// Check if this command is a measurement
    #[must_use]
    pub fn is_measurement(&self) -> bool {
        match self {
            QirCommand::Gate(gate_command) => {
                gate_command.gate_type == pecos_engines::byte_message::GateType::Measure
            }
            QirCommand::Record(_) => false,
        }
    }
}

/// Identifies circuit boundaries by analyzing command patterns
///
/// This function identifies circuit boundaries by looking for measurement patterns
/// in the command sequence. It helps determine which commands should be included
/// in a single circuit execution.
///
/// # Arguments
///
/// * `commands` - The quantum commands to analyze
///
/// # Returns
///
/// * `Vec<QirCommand>` - The commands up to the identified circuit boundary
#[must_use]
pub fn identify_circuit_boundaries(commands: &[QirCommand]) -> Vec<QirCommand> {
    // Identify circuit boundaries by looking for measurement patterns
    let mut measurement_indices = Vec::new();
    let mut gate_indices = Vec::new();

    // First pass: identify measurements and gates
    for (i, cmd) in commands.iter().enumerate() {
        if cmd.is_measurement() {
            measurement_indices.push(i);
        } else if cmd.is_gate() {
            gate_indices.push(i);
        }
    }

    // If we have both gates and measurements, try to find a circuit boundary
    if !gate_indices.is_empty() && !measurement_indices.is_empty() {
        // Find the first set of consecutive measurements at the end of a sequence of gates
        let last_gate_index = *gate_indices.last().unwrap_or(&0);
        let first_measurement_after_gates = measurement_indices
            .iter()
            .find(|&&idx| idx > last_gate_index);

        if let Some(&first_meas_idx) = first_measurement_after_gates {
            // Find consecutive measurements
            let mut end_idx = first_meas_idx;
            for (i, cmd) in commands.iter().enumerate().skip(first_meas_idx) {
                if cmd.is_measurement() {
                    end_idx = i;
                } else {
                    break;
                }
            }

            // Take all commands up to and including the last consecutive measurement
            let circuit_commands = commands[0..=end_idx].to_vec();
            debug!(
                "QIR: Found circuit boundary after {} commands with {} measurements",
                end_idx + 1,
                end_idx - first_meas_idx + 1
            );
            circuit_commands
        } else {
            // If we can't find measurements after gates, take all commands
            commands.to_vec()
        }
    } else {
        // If we don't have both gates and measurements, take all commands
        commands.to_vec()
    }
}

/// Converts a list of `QirCommand`s to a `ByteMessage`
///
/// This function converts a list of `QirCommand`s to a `ByteMessage` that can
/// be processed by the quantum system.
///
/// # Arguments
///
/// * `commands` - The QIR commands to convert
///
/// # Returns
///
/// * `Result<ByteMessage, PecosError>` - The `ByteMessage` if successful, or an error if the operation fails
///
/// # Errors
///
/// Returns an error if the commands cannot be converted to a `ByteMessage`.
pub fn commands_to_byte_message(commands: &[QirCommand]) -> Result<ByteMessage, PecosError> {
    use pecos_engines::ByteMessageBuilder;

    // Get the current thread ID for logging
    let thread_id = get_thread_id();

    debug!(
        "QIR: [Thread {}] Converting {} commands to ByteMessage",
        thread_id,
        commands.len()
    );

    // Convert QirCommands to ByteMessage

    let mut builder = ByteMessageBuilder::new();
    let _ = builder.for_quantum_operations();

    for cmd in commands {
        match cmd {
            QirCommand::Gate(gate_command) => {
                // Directly use the GateCommand
                builder.add_gate_command(gate_command);
            }
            QirCommand::Record(record_data) => {
                // Handle record data
                match record_data {
                    RecordData::ResultRecord(result_id, label) => {
                        builder.add_result_record(*result_id, label.as_deref());
                    }
                    RecordData::KeyValueRecord(key, value) => {
                        builder.add_record_data(key, *value);
                    }
                    RecordData::RawRecord(raw) => {
                        builder.add_debug_message(raw);
                    }
                }
            }
        }
    }

    Ok(builder.build())
}
