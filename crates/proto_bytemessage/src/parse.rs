use bytemuck::Pod;

/// Parses a byte slice into a reference to a value of type `T` without copying.
///
/// # Type Parameters
/// - `T`: The type to parse the byte slice into, which must implement the `Pod` trait.
///
/// # Arguments
/// - `payload`: The byte slice to be parsed.
///
/// # Returns
/// - `Ok(&T)`: A reference to the parsed value if the size matches the expected type.
/// - `Err(String)`: An error message if the payload size is invalid.
///
/// # Errors
/// This function returns an error if the size of the `payload` does not match
/// the size of the type `T`.
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
