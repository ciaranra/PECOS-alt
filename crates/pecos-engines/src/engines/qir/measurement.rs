use crate::byte_message::ByteMessage;
use crate::engines::qir::common::get_thread_id;
use crate::errors::QueueError;
use crate::shot_results::ShotResult;
use log::{debug, trace, warn};
use std::collections::HashMap;

/// Processes measurement results from a `ByteMessage`
///
/// This function extracts measurement results from a `ByteMessage` and stores them
/// in the provided `measurement_results` map.
///
/// # Arguments
///
/// * `message` - The `ByteMessage` containing measurement results
/// * `measurement_results` - The map to store the measurement results in
/// * `shot_count` - The current shot count (for logging)
///
/// # Returns
///
/// * `Result<(), QueueError>` - Ok if successful, or an error if the operation fails
pub fn process_measurements<S: ::std::hash::BuildHasher>(
    message: &ByteMessage,
    measurement_results: &mut HashMap<usize, u32, S>,
    shot_count: usize,
) -> Result<(), QueueError> {
    // Get the current thread ID for logging
    let thread_id = get_thread_id();

    debug!(
        "QIR: [Thread {}] Processing measurements from ByteMessage for shot {}",
        thread_id,
        shot_count + 1
    );

    // Extract measurements from ByteMessage using the binary protocol
    let measurements = message.measurement_results_as_vec().map_err(|e| {
        warn!(
            "QIR: [Thread {}] Failed to extract measurements from ByteMessage: {}",
            thread_id, e
        );
        e
    })?;

    if measurements.is_empty() {
        debug!("QIR: [Thread {}] No measurements to process", thread_id);
        return Ok(());
    }

    debug!(
        "QIR: [Thread {}] Processing {} measurements",
        thread_id,
        measurements.len()
    );

    // Clear previous measurements
    measurement_results.clear();

    // Process all measurements directly into our internal map
    for (result_id, value) in &measurements {
        debug!(
            "QIR: [Thread {}] Received measurement: result_id={}, value={}",
            thread_id, result_id, value
        );

        // Store in our internal map using the numeric result_id directly
        measurement_results.insert(*result_id, *value);
    }

    // Log all measurements after processing
    debug!(
        "QIR: [Thread {}] All measurements after processing:",
        thread_id
    );
    for (result_id, value) in measurement_results {
        debug!("QIR:   result_{} = {}", result_id, value);
    }

    Ok(())
}

/// Creates a `ShotResult` from measurement results
///
/// This function creates a `ShotResult` from the provided `measurement_results` map.
///
/// # Arguments
///
/// * `measurement_results` - The map containing measurement results
///
/// # Returns
///
/// * `ShotResult` - The created `ShotResult`
#[must_use]
pub fn get_results<S: ::std::hash::BuildHasher>(
    measurement_results: &HashMap<usize, u32, S>,
) -> ShotResult {
    // Get the current thread ID for logging
    let thread_id = get_thread_id();

    debug!("QIR: [Thread {}] Getting results", thread_id);

    // Create ShotResult from measurement_results
    let mut shot_result = ShotResult::default();

    // Log all available measurements
    trace!(
        "QIR: [Thread {}] Available measurements for result generation:",
        thread_id
    );
    for (result_id, value) in measurement_results {
        trace!(
            "QIR: [Thread {}]   result_{} = {}",
            thread_id, result_id, value
        );
    }

    // Sort measurements by result_id for consistent ordering
    let mut sorted_result_ids: Vec<_> = measurement_results.keys().collect();
    sorted_result_ids.sort();

    // Use a StringBuilder-like approach for the combined result
    let mut result_bits = Vec::with_capacity(sorted_result_ids.len());

    // Process all measurement results in sorted order
    for &result_id in &sorted_result_ids {
        if let Some(value) = measurement_results.get(result_id) {
            // Add to result bits vector
            trace!(
                "QIR: [Thread {}]   Adding result_{} = {} to combined result",
                thread_id, result_id, value
            );
            result_bits.push(*value != 0);

            // Add to measurements map with numeric ID as key
            // Use a static prefix with the numeric ID to avoid string concatenation
            // Note: ShotResult requires string keys, so we still need to create these strings
            // but we minimize the number of allocations
            let key = format!("result_{result_id}");
            shot_result.measurements.insert(key, *value);
        }
    }

    // Convert bit vector to string only when needed
    if result_bits.is_empty() {
        debug!(
            "QIR: [Thread {}] No measurements available for combined result",
            thread_id
        );
    } else {
        // Create the combined result string directly from the bit vector
        // This avoids intermediate string allocations
        let binary_string =
            result_bits
                .iter()
                .fold(String::with_capacity(result_bits.len()), |mut s, &b| {
                    s.push(if b { '1' } else { '0' });
                    s
                });

        // Set the combined result
        shot_result.combined_result = Some(binary_string.clone());

        // Also add it to the measurements map with the key "result"
        // Convert the binary string to a u32 value if possible, or use 1 for non-zero results
        let result_value = if let Ok(value) = u32::from_str_radix(&binary_string, 2) {
            value
        } else {
            u32::from(binary_string.contains('1'))
        };

        shot_result
            .measurements
            .insert("result".to_string(), result_value);

        debug!(
            "QIR: [Thread {}] Final combined result: {} (value: {})",
            thread_id, binary_string, result_value
        );
    }

    debug!(
        "QIR: [Thread {}] ShotResult: combined_result={:?}, measurements={:?}",
        thread_id, shot_result.combined_result, shot_result.measurements
    );

    shot_result
}
