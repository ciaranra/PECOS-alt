use pecos_engines::byte_message::ByteMessage;

#[test]
fn test_measurement_roundtrip() {
    // Create measurement results using builder
    let mut builder = ByteMessage::outcomes_builder();
    builder.add_outcomes(&[0, 1]);
    let msg = builder.build();

    // Try to parse them back
    match msg.outcomes() {
        Ok(parsed) => println!("Parsed measurements: {parsed:?}"),
        Err(e) => println!("Error parsing: {e:?}"),
    }

    // Create indexed results (similar to what measurement_results_as_vec did)
    match msg.outcomes() {
        Ok(outcomes) => {
            let indexed: Vec<(usize, u32)> = outcomes.into_iter().enumerate().collect();
            println!("Parsed as indexed vec: {indexed:?}");
        }
        Err(e) => println!("Error parsing as indexed vec: {e:?}"),
    }

    // Check message type
    match msg.message_type() {
        Ok(mt) => println!("Message type: {mt:?}"),
        Err(e) => println!("Error getting type: {e:?}"),
    }
}
