use prototype::message::debug::MessageDebug;
use prototype::message::{
    BatchBuilder, GateType, MessageHeader, MessageType, OperationData, ProgramData, QuantumOpData,
};
use prototype::processor::{CompositeProcessor, Processor, ProgramProcessor, SimulatorProcessor};

fn print_message_layouts() {
    use memoffset::offset_of;
    use std::mem::{align_of, size_of};

    println!("\nMessage Structure Details:");

    println!(
        "\nMessageHeader (size={}, align={})",
        size_of::<MessageHeader>(),
        align_of::<MessageHeader>()
    );
    println!("  msg_type offset: {}", offset_of!(MessageHeader, msg_type));
    println!(
        "  payload_size offset: {}",
        offset_of!(MessageHeader, payload_size)
    );

    println!(
        "\nProgramData (size={}, align={})",
        size_of::<ProgramData>(),
        align_of::<ProgramData>()
    );
    println!(
        "  num_operations offset: {}",
        offset_of!(ProgramData, num_operations)
    );

    println!(
        "\nOperationData (size={}, align={})",
        size_of::<OperationData>(),
        align_of::<OperationData>()
    );
    println!(
        "  gate_type offset: {}",
        offset_of!(OperationData, gate_type)
    );
    println!(
        "  num_qubits offset: {}",
        offset_of!(OperationData, num_qubits)
    );

    println!(
        "\nQuantumOpData (size={}, align={})",
        size_of::<QuantumOpData>(),
        align_of::<QuantumOpData>()
    );
    println!(
        "  gate_type offset: {}",
        offset_of!(QuantumOpData, gate_type)
    );
    println!(
        "  num qubits offset: {}",
        offset_of!(QuantumOpData, num_qubits)
    );

    // Print enum sizes
    println!("\nEnum sizes:");
    println!("  MessageType: {}", size_of::<MessageType>());
    println!("  GateType: {}", size_of::<GateType>());

    // Print type details
    println!("\nCore type details:");
    println!("  u8: size={}, align={}", size_of::<u8>(), align_of::<u8>());
    println!(
        "  u16: size={}, align={}",
        size_of::<u16>(),
        align_of::<u16>()
    );
    println!(
        "  u32: size={}, align={}",
        size_of::<u32>(),
        align_of::<u32>()
    );

    // Add hex dump helper
    println!("\nHex dump helper:");
    println!("  00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f");
    println!("  -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- --");
}

// Example usage
fn main() {
    // Print structure layouts
    print_message_layouts();

    // Create initial batch with program
    let mut builder = BatchBuilder::new();

    // Add program (Bell state + measurement)
    let operations = vec![
        (GateType::H, vec![0]),
        (GateType::CX, vec![0, 1]),
        (GateType::Measure, vec![0]),
        (GateType::Measure, vec![1]),
    ];

    println!("\nCreating program batch...");

    // Add program data to an Input message
    builder.add_message(MessageType::Input, &[]);
    println!("Added Input message header");

    builder.add_program(&operations);
    println!("Added program data");

    let initial_batch = builder.build();
    println!("Built initial batch\n");

    // Trace initial program
    MessageDebug::trace_batch(&initial_batch, "Initial Program");

    // Create processors
    let program_processor = ProgramProcessor::new();
    let sim_processor = SimulatorProcessor::new(2);

    // Create mediator and run
    let mut mediator = CompositeProcessor {
        processor1: program_processor,
        processor2: sim_processor,
    };

    // println!("\nRunning program...\n");
    // let result = mediator.process(initial_batch);
    //
    // // Trace final results
    // MessageDebug::trace_batch(&result, "Final Results");
    //
    // println!("\nFinal measurement results:");
    // print_results(&result);
    // println!("Done.");

    println!("\nRunning program...\n");
    let result = mediator.process(initial_batch);

    // Trace final results
    MessageDebug::trace_batch(&result, "Final Results");
    MessageDebug::dump_measurement_results(&result);
    println!("Done.");
}
