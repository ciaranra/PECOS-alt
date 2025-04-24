use pecos_engines::byte_message::ByteMessage;
use pecos_engines::engines::quantum::StateVecEngine;
use pecos_engines::{Engine, GeneralDepolarizingNoise};
use pecos_engines::{EngineSystem, QuantumSystem};

fn main() {
    let circ = ByteMessage::quantum_operations_builder()
        .add_h(&[0])
        .add_cx(&[0], &[1])
        .add_measurements(&[0], &[0])
        .add_measurements(&[1], &[1])
        .build();

    let quantum = Box::new(StateVecEngine::new(2));
    let noise = Box::new(GeneralDepolarizingNoise::new(0.1, 0.1, 0.1, 0.1));

    let mut system = QuantumSystem::new(noise, quantum);

    // system.set_seed(42).expect("failed to set seed");

    print!("[");
    for _ in 0..20 {
        system.reset().expect("failed to reset");
        let results = system
            .process_as_system(circ.clone())
            .expect("failed to process circ");
        let meas = results
            .parse_measurements()
            .expect("failed to parse measurements");

        print!("\"");
        for &(_, value) in &meas {
            print!("{value}");
        }
        print!("\", ");
    }
    print!("]");

    println!();
}
