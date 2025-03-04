use crate::message::ptr::AlignedCast;
use crate::message::types::{
    MeasResultData, MessageBatch, MessageHeader, MessageType, OperationData, ProgramData,
    QuantumOpData,
};

#[allow(clippy::cast_ptr_alignment)]
pub struct MessageDebug;

impl MessageDebug {
    // Helper to calculate aligned offset
    fn align_offset(current: usize, alignment: usize) -> usize {
        if alignment == 0 {
            return 0;
        }
        let misalignment = current % alignment;
        if misalignment == 0 {
            0
        } else {
            alignment - misalignment
        }
    }

    // Split message dumping into smaller functions for better organization
    #[allow(clippy::cast_sign_loss)]
    unsafe fn dump_header(ptr: *const u8) -> MessageHeader {
        unsafe {
            let msg_type = std::mem::transmute::<u8, MessageType>(*ptr);
            let payload_size = u16::from_le_bytes([*ptr.add(2), *ptr.add(3)]);
            MessageHeader {
                msg_type,
                payload_size,
            }
        }
    }

    fn visualize_bytes(data: &[u8], start: usize, size: usize, indent: usize, label: &str) {
        print!("{:indent$}", "", indent = indent);
        println!("{label}:");
        print!("{:indent$}", "", indent = indent + 2);
        println!("Offset: 0x{start:04x}, Size: {size} bytes");

        for row in 0..(size + 15) / 16 {
            print!(
                "{:indent$}0x{:04x}:  ",
                "",
                start + row * 16,
                indent = indent + 2
            );

            // Print hex values
            for col in 0..16 {
                let pos = row * 16 + col;
                if pos < size {
                    print!("{:02x} ", data[start + pos]);
                } else {
                    print!("   ");
                }
            }

            // Print ASCII representation
            print!(" | ");
            for col in 0..16 {
                let pos = row * 16 + col;
                if pos < size {
                    let c = data[start + pos];
                    if (32..=126).contains(&c) {
                        print!("{}", c as char);
                    } else {
                        print!(".");
                    }
                } else {
                    print!(" ");
                }
            }
            println!();
        }
    }

    unsafe fn dump_operation(
        data: &[u8],
        op_ptr: *const u8,
        op_offset: usize,
        indent: usize,
        next_op_ptr: &mut *const u8,
        next_op_offset: &mut usize,
    ) {
        unsafe {
            Self::visualize_bytes(
                data,
                op_offset,
                std::mem::size_of::<OperationData>(),
                indent,
                "Operation Header",
            );

            let op = &*op_ptr.cast_aligned::<OperationData>();
            println!(
                "{:indent$}Gate Type: {:?}",
                "",
                op.gate_type,
                indent = indent
            );
            println!(
                "{:indent$}Num Qubits: {}",
                "",
                op.num_qubits,
                indent = indent
            );

            *next_op_ptr = op_ptr.add(std::mem::size_of::<OperationData>());
            *next_op_offset = op_offset + std::mem::size_of::<OperationData>();

            if op.num_qubits > 0 {
                let qubit_size = op.num_qubits as usize * std::mem::size_of::<u32>();
                Self::visualize_bytes(data, *next_op_offset, qubit_size, indent, "Qubit Indices");

                let qubits = std::slice::from_raw_parts(
                    (*next_op_ptr).cast_aligned::<u32>(),
                    op.num_qubits as usize,
                );
                print!("{:indent$}Qubits: ", "", indent = indent);
                for &qubit in qubits {
                    print!("{qubit} ");
                }
                println!();

                *next_op_ptr = next_op_ptr.add(qubit_size);
                *next_op_offset += qubit_size;
            }
        }
    }

    unsafe fn dump_program_data(data: &[u8], ptr: *const u8, indent: usize) {
        unsafe {
            let program = &*ptr.cast_aligned::<ProgramData>();
            println!("{:indent$}Program:", "", indent = indent + 4);
            println!(
                "{:indent$}Operations: {}",
                "",
                program.num_operations,
                indent = indent + 6
            );

            let mut op_ptr = ptr.add(std::mem::size_of::<ProgramData>());
            let mut op_offset = std::mem::size_of::<ProgramData>();

            for i in 0..program.num_operations {
                println!("\n{:indent$}Operation {}:", "", i, indent = indent + 6);
                Self::dump_operation(
                    data,
                    op_ptr,
                    op_offset,
                    indent + 8,
                    &mut op_ptr,
                    &mut op_offset,
                );
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    unsafe fn dump_next_message(
        current: *const u8,
        remaining_size: usize,
        indent: usize,
    ) -> *const u8 {
        unsafe {
            if remaining_size < std::mem::size_of::<MessageHeader>() {
                println!(
                    "{:indent$}WARNING: Not enough bytes for header",
                    "",
                    indent = indent
                );
                return current.add(remaining_size);
            }

            let data = std::slice::from_raw_parts(current, remaining_size);
            println!(
                "\n{:indent$}=== Message at offset 0x{:04x} ===",
                "",
                current as usize,
                indent = indent
            );

            // Visualize and parse header
            Self::visualize_bytes(
                data,
                0,
                std::mem::size_of::<MessageHeader>(),
                indent,
                "MessageHeader",
            );
            let header = Self::dump_header(current);

            println!(
                "{:indent$}Message Type: {:?} ({:02x})",
                "",
                header.msg_type,
                data[0],
                indent = indent + 2
            );
            println!(
                "{:indent$}Payload Size: {} bytes ({:02x} {:02x})",
                "",
                header.payload_size,
                data[2],
                data[3],
                indent = indent + 2
            );

            let mut next_ptr = current.add(std::mem::size_of::<MessageHeader>());
            let mut payload_offset = std::mem::size_of::<MessageHeader>();

            // Handle alignment
            let align = match header.msg_type {
                MessageType::Input => std::mem::align_of::<ProgramData>(),
                MessageType::QuantumOp => std::mem::align_of::<QuantumOpData>(),
                MessageType::MeasResult => std::mem::align_of::<MeasResultData>(),
                _ => 1,
            };

            if align > 1 {
                let align_padding = Self::align_offset(payload_offset, align);
                if align_padding > 0 {
                    println!(
                        "{:indent$}Alignment padding: {} bytes",
                        "",
                        align_padding,
                        indent = indent + 2
                    );
                    next_ptr = next_ptr.add(align_padding);
                    payload_offset += align_padding;
                }
            }

            // Process payload
            if header.payload_size > 0 {
                println!(
                    "{:indent$}Payload at offset 0x{:04x}:",
                    "",
                    payload_offset,
                    indent = indent + 2
                );
                Self::visualize_bytes(
                    data,
                    payload_offset,
                    header.payload_size as usize,
                    indent + 2,
                    "Payload Data",
                );

                if header.msg_type == MessageType::Input {
                    Self::dump_program_data(data, next_ptr, indent);
                } else {
                    print!("{:indent$}Raw bytes: ", "", indent = indent + 4);
                    for i in 0..header.payload_size {
                        print!("{:02x} ", *next_ptr.add(i as usize));
                    }
                    println!();
                }
            }

            println!("{:indent$}=== End Message ===\n", "", indent = indent);
            current.add(std::mem::size_of::<MessageHeader>() + header.payload_size as usize)
        }
    }

    // Main entry point - dump entire message batch
    pub fn dump_batch(batch: &MessageBatch) {
        println!("=== Message Batch ===");
        println!("Total size: {} bytes", batch.total_size);

        // Print raw bytes in debug format
        println!("\nRaw bytes (first 64):");
        for i in 0..std::cmp::min(64, batch.total_size as usize) {
            if i % 16 == 0 {
                println!("  ");
            }
            print!("{:02x} ", unsafe { *batch.data.add(i) });
        }
        println!("\n");

        println!("Message contents:");
        Self::dump_messages(batch);
    }

    // Process all messages in a batch
    #[allow(clippy::cast_sign_loss)]
    fn dump_messages(batch: &MessageBatch) {
        unsafe {
            let mut current = batch.data;
            let mut remaining = batch.total_size as usize;

            while remaining > std::mem::size_of::<MessageHeader>() {
                let next = Self::dump_next_message(current, remaining, 0);
                if next <= current {
                    println!("WARNING: Message parser not advancing!");
                    break;
                }

                let advance = next.offset_from(current) as usize;
                current = next;
                remaining = remaining.saturating_sub(advance);
            }

            if remaining > 0 {
                println!("WARNING: {remaining} bytes remaining after parsing");
            }
        }
    }

    // Add the trace_batch function
    pub fn trace_batch(batch: &MessageBatch, location: &str) {
        println!("\n=== Message Batch at {location} ===");
        Self::dump_batch(batch);
        println!("=== End of Batch ===\n");
    }

    pub fn dump_measurement_results(batch: &MessageBatch) {
        println!("\n=== Measurement Results in Batch ===");

        unsafe {
            let mut current = batch.data;
            let end = current.add(batch.total_size as usize);
            let mut count = 0;

            while current < end {
                let header = &*current.cast_aligned::<MessageHeader>();
                current = current.add(std::mem::size_of::<MessageHeader>());

                match header.msg_type {
                    MessageType::MeasResult => {
                        count += 1;
                        let result = &*current.cast_aligned::<MeasResultData>();
                        println!(
                            "Measurement {}: Qubit {} -> {}",
                            count,
                            result.qubit,
                            if result.outcome { "|1⟩" } else { "|0⟩" }
                        );
                        current = current.add(std::mem::size_of::<MeasResultData>());
                    }
                    _ => {
                        current = current.add(header.payload_size as usize);
                    }
                }
            }

            println!("Total measurements: {count}");
        }

        println!("=== End Measurement Results ===\n");
    }
}
