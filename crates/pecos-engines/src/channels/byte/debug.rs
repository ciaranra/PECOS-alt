//! Utilities for debugging and inspecting byte protocol messages
//!
//! This module provides functions for dumping and analyzing binary
//! messages for debugging purposes.

use super::builder::MessageBuilder;
use super::channel::ByteChannel;
use super::protocol::{
    BATCH_MAGIC, BatchHeader, MeasurementHeader, MeasurementResultHeader, MessageHeader,
    QuantumGateHeader, calc_padding,
};
use crate::channels::CommandChannel;
use crate::errors::QueueError;
use pecos_core::types::CommandBatch;
use std::io::{Cursor, Write};
use std::mem::size_of;

/// Dump a binary message batch to a string for debugging
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn dump_batch(data: &[u8]) -> String {
    let mut output = String::new();

    // Check if we have enough bytes for a batch header
    if data.len() < size_of::<BatchHeader>() {
        output.push_str("ERROR: Data too small for batch header\n");
        return output;
    }

    // Parse batch header
    let header = *bytemuck::from_bytes::<BatchHeader>(&data[0..size_of::<BatchHeader>()]);

    if header.magic != BATCH_MAGIC {
        output.push_str(&format!(
            "ERROR: Invalid magic number: 0x{:08x} (expected 0x{:08x})\n",
            header.magic, BATCH_MAGIC
        ));
        return output;
    }

    output.push_str("Batch Header:\n");
    output.push_str(&format!("  Magic: 0x{:08x}\n", header.magic));
    output.push_str(&format!("  Version: {}\n", header.version));
    output.push_str(&format!("  Flags: 0x{:02x}\n", header.flags));
    output.push_str(&format!("  Message Count: {}\n", header.msg_count));
    output.push_str(&format!("  Total Size: {} bytes\n", header.total_size));
    output.push('\n');

    // Parse each message
    let mut offset = size_of::<BatchHeader>();
    for i in 0..header.msg_count {
        // Ensure we have enough data for a message header
        if offset + size_of::<MessageHeader>() > data.len() {
            output.push_str(&format!("ERROR: Data too small for message {i} header\n"));
            break;
        }

        // Parse message header
        let msg_header = *bytemuck::from_bytes::<MessageHeader>(
            &data[offset..offset + size_of::<MessageHeader>()],
        );

        offset += size_of::<MessageHeader>();

        // Get message type
        let msg_type = match msg_header.msg_type {
            1 => "BeginBatch",
            2 => "EndBatch",
            3 => "Flush",
            4 => "Reset",
            10 => "QuantumGate",
            11 => "Measurement",
            20 => "MeasurementResult",
            100 => "Error",
            _ => "Unknown",
        };

        output.push_str(&format!("Message {i}:\n"));
        output.push_str(&format!("  Type: {} ({})\n", msg_type, msg_header.msg_type));
        output.push_str(&format!("  Flags: 0x{:02x}\n", msg_header.flags));
        output.push_str(&format!(
            "  Payload Size: {} bytes\n",
            msg_header.payload_size
        ));

        // Parse payload based on message type
        if msg_header.payload_size > 0 {
            let payload_end = offset + msg_header.payload_size as usize;
            if payload_end > data.len() {
                output.push_str(&format!("ERROR: Data too small for message {i} payload\n"));
                break;
            }

            let payload = &data[offset..payload_end];

            match msg_header.msg_type {
                10 => {
                    // QuantumGate
                    if payload.len() >= size_of::<QuantumGateHeader>() {
                        let gate_header = *bytemuck::from_bytes::<QuantumGateHeader>(
                            &payload[0..size_of::<QuantumGateHeader>()],
                        );

                        let gate_type = match gate_header.gate_type {
                            1 => "X",
                            2 => "Y",
                            3 => "Z",
                            4 => "H",
                            5 => "CX",
                            6 => "RZ",
                            7 => "R1XY",
                            8 => "SZZ",
                            _ => "Unknown",
                        };

                        output.push_str("  Quantum Gate:\n");
                        output.push_str(&format!(
                            "    Type: {} ({})\n",
                            gate_type, gate_header.gate_type
                        ));
                        output.push_str(&format!("    Qubits: {}\n", gate_header.num_qubits));
                        output.push_str(&format!(
                            "    Has Parameters: {}\n",
                            gate_header.has_params != 0
                        ));

                        // Dump qubit indices
                        let qubits_offset = size_of::<QuantumGateHeader>();
                        let mut qubits = Vec::new();

                        for i in 0..gate_header.num_qubits as usize {
                            let offset = qubits_offset + i * size_of::<u32>();
                            if offset + size_of::<u32>() <= payload.len() {
                                let qubit = u32::from_le_bytes([
                                    payload[offset],
                                    payload[offset + 1],
                                    payload[offset + 2],
                                    payload[offset + 3],
                                ]);
                                qubits.push(qubit);
                            }
                        }

                        output.push_str(&format!("    Qubit Indices: {qubits:?}\n"));

                        // Dump parameters if present
                        if gate_header.has_params != 0 {
                            let params_offset =
                                qubits_offset + gate_header.num_qubits as usize * size_of::<u32>();

                            match gate_header.gate_type {
                                6 => {
                                    // RZ
                                    if params_offset + size_of::<f64>() <= payload.len() {
                                        let theta = f64::from_le_bytes([
                                            payload[params_offset],
                                            payload[params_offset + 1],
                                            payload[params_offset + 2],
                                            payload[params_offset + 3],
                                            payload[params_offset + 4],
                                            payload[params_offset + 5],
                                            payload[params_offset + 6],
                                            payload[params_offset + 7],
                                        ]);

                                        output.push_str(&format!("    Theta: {theta}\n"));
                                    }
                                }
                                7 => {
                                    // R1XY
                                    if params_offset + 2 * size_of::<f64>() <= payload.len() {
                                        let phi = f64::from_le_bytes([
                                            payload[params_offset],
                                            payload[params_offset + 1],
                                            payload[params_offset + 2],
                                            payload[params_offset + 3],
                                            payload[params_offset + 4],
                                            payload[params_offset + 5],
                                            payload[params_offset + 6],
                                            payload[params_offset + 7],
                                        ]);

                                        let theta = f64::from_le_bytes([
                                            payload[params_offset + 8],
                                            payload[params_offset + 9],
                                            payload[params_offset + 10],
                                            payload[params_offset + 11],
                                            payload[params_offset + 12],
                                            payload[params_offset + 13],
                                            payload[params_offset + 14],
                                            payload[params_offset + 15],
                                        ]);

                                        output.push_str(&format!("    Phi: {phi}\n"));
                                        output.push_str(&format!("    Theta: {theta}\n"));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                11 => {
                    // Measurement
                    if payload.len() >= size_of::<MeasurementHeader>() {
                        let meas_header = *bytemuck::from_bytes::<MeasurementHeader>(
                            &payload[0..size_of::<MeasurementHeader>()],
                        );

                        output.push_str("  Measurement:\n");
                        output.push_str(&format!("    Qubit: {}\n", meas_header.qubit));
                        output.push_str(&format!("    Result ID: {}\n", meas_header.result_id));
                    }
                }
                20 => {
                    // MeasurementResult
                    if payload.len() >= size_of::<MeasurementResultHeader>() {
                        let result_header = *bytemuck::from_bytes::<MeasurementResultHeader>(
                            &payload[0..size_of::<MeasurementResultHeader>()],
                        );

                        output.push_str("  Measurement Result:\n");
                        output.push_str(&format!("    Result ID: {}\n", result_header.result_id));
                        output.push_str(&format!("    Outcome: {}\n", result_header.outcome));
                    }
                }
                _ => {
                    // Dump raw bytes for other message types
                    output.push_str(&format!("  Payload: {payload:?}\n"));
                }
            }

            offset = payload_end;
        }

        // Skip padding to next message
        let padding = calc_padding(msg_header.payload_size as usize, 4);
        if padding > 0 {
            offset += padding;
        }

        output.push('\n');
    }

    output
}

/// Convert a batch to a binary message for inspection
#[must_use]
pub fn batch_to_binary(batch: &CommandBatch) -> Vec<u8> {
    let mut builder = MessageBuilder::new();
    builder.add_command_batch(batch).build()
}

/// Decode a binary message back to a command batch
pub fn binary_to_batch(data: &[u8]) -> Result<CommandBatch, QueueError> {
    let owned_data = data.to_vec();
    let mut channel =
        ByteChannel::new(Box::new(Cursor::new(owned_data)), Box::new(std::io::sink()));

    match channel.receive_batch()? {
        Some(batch) => Ok(batch),
        None => Ok(CommandBatch::new()),
    }
}

/// Utility function to write a byte dump to a file for debugging
pub fn write_batch_to_file(batch: &CommandBatch, filename: &str) -> std::io::Result<()> {
    let binary_data = batch_to_binary(batch);
    let mut file = std::fs::File::create(filename)?;
    file.write_all(&binary_data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pecos_core::types::{GateType, QuantumCommand};

    #[test]
    fn test_batch_to_binary_to_batch() {
        // Create a command batch
        let mut batch = CommandBatch::new();
        batch.add_command(QuantumCommand {
            gate: GateType::H,
            qubits: vec![0],
        });
        batch.add_command(QuantumCommand {
            gate: GateType::CX,
            qubits: vec![0, 1],
        });

        // Convert to binary
        let binary_data = batch_to_binary(&batch);

        // Convert back to batch
        let recovered_batch = binary_to_batch(&binary_data).unwrap();

        // Verify
        assert_eq!(recovered_batch.len(), 2);

        let commands: Vec<_> = recovered_batch.commands().iter().collect();

        assert!(matches!(commands[0].gate, GateType::H));
        assert_eq!(commands[0].qubits.len(), 1);
        assert_eq!(commands[0].qubits[0], 0);

        assert!(matches!(commands[1].gate, GateType::CX));
        assert_eq!(commands[1].qubits.len(), 2);
        assert_eq!(commands[1].qubits[0], 0);
        assert_eq!(commands[1].qubits[1], 1);
    }

    #[test]
    fn test_dump_batch() {
        // Create a command batch with different gate types
        let mut batch = CommandBatch::new();
        batch.add_command(QuantumCommand {
            gate: GateType::H,
            qubits: vec![0],
        });
        batch.add_command(QuantumCommand {
            gate: GateType::RZ { theta: 0.5 },
            qubits: vec![1],
        });

        // Convert to binary
        let binary_data = batch_to_binary(&batch);

        // Dump batch
        let dump = dump_batch(&binary_data);

        // Verify dump contains expected information
        assert!(dump.contains("Batch Header"));
        assert!(dump.contains("Magic: 0x5045"));
        assert!(dump.contains("Type: QuantumGate"));
        assert!(dump.contains("Type: H"));
        assert!(dump.contains("Type: RZ"));
        assert!(dump.contains("Theta: 0.5"));
    }
}
