use pecos_engines::byte_message::ByteMessage;

#[test]
fn test_measurement_roundtrip() {
    // Create measurement results
    let measurements = vec![(0, 0u32), (1, 1u32)];
    let msg = ByteMessage::record_measurement_results(&measurements);

    // Try to parse them back
    match msg.parse_measurements() {
        Ok(parsed) => println!("Parsed measurements: {parsed:?}"),
        Err(e) => println!("Error parsing: {e:?}"),
    }

    match msg.measurement_results_as_vec() {
        Ok(parsed) => println!("Parsed as vec: {parsed:?}"),
        Err(e) => println!("Error parsing as vec: {e:?}"),
    }

    // Check message type
    match msg.message_type() {
        Ok(mt) => println!("Message type: {mt:?}"),
        Err(e) => println!("Error getting type: {e:?}"),
    }
}
