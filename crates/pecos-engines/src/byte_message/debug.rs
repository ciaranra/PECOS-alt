//! Utilities for debugging and inspecting byte protocol messages
//!
//! This module provides functions for dumping and analyzing binary
//! messages for debugging purposes.

use crate::byte_message::message::ByteMessage;
use crate::byte_message::protocol::{
    BATCH_MAGIC, BatchHeader, MeasurementHeader, MeasurementResultHeader, MessageHeader,
    QuantumGateHeader, calc_padding,
};
use std::fmt::Write;
use std::io::Write as IoWrite;
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
        writeln!(
            output,
            "ERROR: Invalid magic number: 0x{:08x} (expected 0x{:08x})",
            header.magic, BATCH_MAGIC
        )
        .unwrap();
        return output;
    }

    output.push_str("Batch Header:\n");
    writeln!(output, "  Magic: 0x{:08x}", header.magic).unwrap();
    writeln!(output, "  Version: {}", header.version).unwrap();
    writeln!(output, "  Flags: 0x{:02x}", header.flags).unwrap();
    writeln!(output, "  Message Count: {}", header.msg_count).unwrap();
    writeln!(output, "  Total Size: {} bytes", header.total_size).unwrap();
    output.push('\n');

    // Parse each message
    let mut offset = size_of::<BatchHeader>();
    for i in 0..header.msg_count {
        // Ensure we have enough data for a message header
        if offset + size_of::<MessageHeader>() > data.len() {
            writeln!(output, "ERROR: Data too small for message {i} header").unwrap();
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

        writeln!(output, "Message {i}:").unwrap();
        writeln!(output, "  Type: {} ({})", msg_type, msg_header.msg_type).unwrap();
        writeln!(output, "  Flags: 0x{:02x}", msg_header.flags).unwrap();
        writeln!(output, "  Payload Size: {} bytes", msg_header.payload_size).unwrap();

        // Parse payload based on message type
        if msg_header.payload_size > 0 {
            let payload_end = offset + msg_header.payload_size as usize;
            if payload_end > data.len() {
                writeln!(output, "ERROR: Data too small for message {i} payload").unwrap();
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
                            6 => "SZZ",
                            7 => "RZ",
                            8 => "R1XY",
                            _ => "Unknown",
                        };

                        output.push_str("  Quantum Gate:\n");
                        writeln!(
                            output,
                            "    Type: {} ({})",
                            gate_type, gate_header.gate_type
                        )
                        .unwrap();
                        writeln!(output, "    Qubits: {}", gate_header.num_qubits).unwrap();
                        writeln!(
                            output,
                            "    Has Parameters: {}",
                            gate_header.has_params != 0
                        )
                        .unwrap();

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

                        writeln!(output, "    Qubit Indices: {qubits:?}").unwrap();

                        // Dump parameters if present
                        if gate_header.has_params != 0 {
                            let params_offset =
                                qubits_offset + gate_header.num_qubits as usize * size_of::<u32>();

                            match gate_header.gate_type {
                                7 => {
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

                                        writeln!(output, "    Theta: {theta}").unwrap();
                                    }
                                }
                                8 => {
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

                                        writeln!(output, "    Phi: {phi}").unwrap();
                                        writeln!(output, "    Theta: {theta}").unwrap();
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
                        writeln!(output, "    Qubit: {}", meas_header.qubit).unwrap();
                    }
                }
                20 => {
                    // MeasurementResult
                    if payload.len() >= size_of::<MeasurementResultHeader>() {
                        let result_header = *bytemuck::from_bytes::<MeasurementResultHeader>(
                            &payload[0..size_of::<MeasurementResultHeader>()],
                        );

                        output.push_str("  Measurement Result:\n");
                        writeln!(output, "    Outcome: {}", result_header.outcome).unwrap();
                    }
                }
                _ => {
                    // Dump raw bytes for other message types
                    writeln!(output, "  Payload: {payload:?}").unwrap();
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

/// Dump a `ByteMessage` to a string for debugging
#[must_use]
pub fn dump_message(message: &ByteMessage) -> String {
    dump_batch(message.as_bytes())
}

/// Utility function to write a `ByteMessage` to a file for debugging
///
/// # Errors
///
/// Returns an error if the file cannot be created or written to.
pub fn write_message_to_file(message: &ByteMessage, filename: &str) -> std::io::Result<()> {
    let mut file = std::fs::File::create(filename)?;
    file.write_all(message.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byte_message::ByteMessage;
    use crate::byte_message::QuantumGate;

    #[test]
    fn test_bytemap_dump() {
        // Create commands
        let commands = vec![QuantumGate::h(0), QuantumGate::cx(0, 1)];

        // Create ByteMessage using the builder pattern
        let message = ByteMessage::builder().add_quantum_gates(&commands).build();

        // Dump the message
        let dump = dump_message(&message);
        println!("{dump}");

        // Verify the dump contains expected information
        assert!(dump.contains("Batch Header"));
        assert!(dump.contains("Quantum Gate"));
    }

    #[test]
    fn test_dump_batch() {
        // Create a ByteMessage with different gate types
        let commands = vec![QuantumGate::h(0), QuantumGate::rz(0.5, 1)];

        // Create a ByteMessage using the builder
        let message = ByteMessage::builder().add_quantum_gates(&commands).build();

        // Dump batch
        let dump = dump_batch(message.as_bytes());

        // Verify dump contains expected information
        assert!(dump.contains("Batch Header"));
        assert!(dump.contains("Magic: 0x5045"));
        assert!(dump.contains("Type: QuantumGate"));
        assert!(dump.contains("Type: H"));
        assert!(dump.contains("Type: RZ"));
        assert!(dump.contains("Theta: 0.5"));
    }
}
