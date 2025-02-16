use bytemuck::Pod;

#[allow(dead_code)]
pub fn parse_message<T: Pod>(payload: &[u8]) -> Result<&T, String> {
    let expected_size = size_of::<T>();
    let actual_size = payload.len();

    if actual_size == expected_size {
        Ok(bytemuck::from_bytes::<T>(payload)) // Zero-copy reinterpretation
    } else {
        Err(format!(
            "Invalid payload size: expected {expected_size}, got {actual_size}"
        ))
    }
}
