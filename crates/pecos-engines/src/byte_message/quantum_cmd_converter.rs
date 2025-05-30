//! Conversion utilities for `QuantumCmd` to `ByteMessage`
//!
//! This module provides the `ByteMessage` conversion functionality for `QuantumCmd`,
//! keeping the core `quantum_cmd.rs` minimal for use in the QIR runtime.

use crate::byte_message::gate_type::GateType;
use crate::byte_message::protocol::{MessageFlags, MessageType};
use crate::byte_message::{ByteMessage, ByteMessageBuilder, QuantumCmd};
use crate::core::record_data::RecordData;
use log::debug;
use pecos_core::errors::PecosError;

/// Extension trait for `QuantumCmd` that provides `ByteMessage` conversion functionality
pub trait QuantumCmdConverter {
    /// Get the `GateType` for this command
    fn gate_type_id(&self) -> Option<GateType>;

    /// Check if this command is a gate operation
    fn is_gate(&self) -> bool;

    /// Check if this command is supported for quantum processing
    fn is_supported(&self) -> bool;

    /// Add this command directly to a `ByteMessageBuilder`
    ///
    /// # Errors
    ///
    /// Returns an error if the command cannot be added to the builder.
    fn add_to_builder(&self, builder: &mut ByteMessageBuilder) -> Result<(), PecosError>;

    /// Convert the command to a `ByteMessage`
    /// This is more efficient than string-based serialization for gate operations
    ///
    /// # Errors
    ///
    /// Returns an error if the command cannot be converted to a byte message.
    fn to_byte_message(&self) -> Result<ByteMessage, PecosError>;

    /// Convert a list of `QuantumCmds` to a `ByteMessage`
    /// This handles all command types, including gate operations, records, and messages
    ///
    /// # Errors
    ///
    /// Returns an error if any command cannot be converted to a byte message.
    fn commands_to_byte_message(commands: &[QuantumCmd]) -> Result<ByteMessage, PecosError>;
}

impl QuantumCmdConverter for QuantumCmd {
    fn gate_type_id(&self) -> Option<GateType> {
        match self {
            QuantumCmd::H(_) => Some(GateType::H),
            QuantumCmd::X(_) => Some(GateType::X),
            QuantumCmd::Y(_) => Some(GateType::Y),
            QuantumCmd::Z(_) => Some(GateType::Z),
            QuantumCmd::CX(_, _) => Some(GateType::CX),
            QuantumCmd::RZ(_, _) => Some(GateType::RZ),
            QuantumCmd::R1XY(_, _, _) => Some(GateType::R1XY),
            QuantumCmd::U(_, _, _, _) => Some(GateType::U),
            QuantumCmd::SZZ(_, _) => Some(GateType::SZZ),
            QuantumCmd::RZZ(_, _, _) => Some(GateType::RZZ),
            QuantumCmd::Measure(_) => Some(GateType::Measure),
            QuantumCmd::Prep(_) => Some(GateType::Prep),
            _ => None,
        }
    }

    fn is_gate(&self) -> bool {
        self.gate_type_id().is_some()
    }

    fn is_supported(&self) -> bool {
        self.is_gate()
    }

    fn add_to_builder(&self, builder: &mut ByteMessageBuilder) -> Result<(), PecosError> {
        match self {
            QuantumCmd::H(qubit) => {
                builder.add_h(&[qubit.0]);
                Ok(())
            }
            QuantumCmd::X(qubit) => {
                builder.add_x(&[qubit.0]);
                Ok(())
            }
            QuantumCmd::Y(qubit) => {
                builder.add_y(&[qubit.0]);
                Ok(())
            }
            QuantumCmd::Z(qubit) => {
                builder.add_z(&[qubit.0]);
                Ok(())
            }
            QuantumCmd::CX(control, target) => {
                builder.add_cx(&[control.0], &[target.0]);
                Ok(())
            }
            QuantumCmd::RZ(angle, qubit) => {
                builder.add_rz(*angle, &[qubit.0]);
                Ok(())
            }
            QuantumCmd::R1XY(theta, phi, qubit) => {
                builder.add_r1xy(*theta, *phi, &[qubit.0]);
                Ok(())
            }
            QuantumCmd::U(theta, phi, lambda, qubit) => {
                builder.add_u(*theta, *phi, *lambda, &[qubit.0]);
                Ok(())
            }
            QuantumCmd::SZZ(qubit1, qubit2) => {
                builder.add_szz(&[qubit1.0], &[qubit2.0]);
                Ok(())
            }
            QuantumCmd::RZZ(angle, qubit1, qubit2) => {
                builder.add_rzz(*angle, &[qubit1.0], &[qubit2.0]);
                Ok(())
            }
            QuantumCmd::Measure(qubit) => {
                builder.add_measurements(&[qubit.0]);
                Ok(())
            }
            QuantumCmd::Prep(qubit) => {
                builder.add_prep(&[qubit.0]);
                Ok(())
            }
            QuantumCmd::Record(data) => {
                match data {
                    RecordData::ResultRecord(result_id, label) => {
                        builder.add_result_record(*result_id, label.as_deref());
                        debug!("Added ResultRecord to ByteMessageBuilder");
                    }
                    RecordData::KeyValueRecord(key, value) => {
                        builder.add_record_data(key, *value);
                        debug!("Added KeyValueRecord to ByteMessageBuilder");
                    }
                    RecordData::RawRecord(cmd) => {
                        // For raw records, we still need to use the string representation
                        let payload = cmd.clone().into_bytes();
                        builder.add_message(MessageType::RecordData, &payload, MessageFlags::NONE);
                        debug!("Added RawRecord to ByteMessageBuilder");
                    }
                }
                Ok(())
            }
            QuantumCmd::Unknown(cmd) => {
                // For unknown commands, use an error message
                let error_msg = cmd.to_string();
                builder.add_error_message(&error_msg);
                debug!("Added Unknown command as error message to ByteMessageBuilder");
                Ok(())
            }
        }
    }

    fn to_byte_message(&self) -> Result<ByteMessage, PecosError> {
        let mut builder = ByteMessage::quantum_operations_builder();
        self.add_to_builder(&mut builder)?;
        Ok(builder.build())
    }

    fn commands_to_byte_message(commands: &[QuantumCmd]) -> Result<ByteMessage, PecosError> {
        let mut builder = ByteMessage::quantum_operations_builder();

        for cmd in commands {
            cmd.add_to_builder(&mut builder)?;
        }

        Ok(builder.build())
    }
}
