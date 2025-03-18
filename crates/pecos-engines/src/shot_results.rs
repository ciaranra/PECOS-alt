use crate::byte_message::ByteMessage;
use crate::errors::QueueError;
use std::collections::HashMap;
use std::fmt;

/// Represents the results of a single shot (execution) of a quantum program.
///
/// This struct contains a mapping of register names to measurement outcomes.
/// Each measurement outcome is represented as a u32 value.
#[derive(Debug, Clone, Default)]
pub struct ShotResult {
    pub measurements: HashMap<String, u32>,
    pub combined_result: Option<String>,
}

impl ShotResult {
    /// Create a `ShotResult` directly from a `ByteMessage` containing measurement results.
    ///
    /// This method extracts measurement results from a `ByteMessage` and creates a `ShotResult`
    /// with properly mapped result IDs to names.
    ///
    /// # Parameters
    ///
    /// * `message` - A `ByteMessage` containing measurement results
    /// * `result_id_to_name` - A mapping from `result_id` to a human-readable name
    ///
    /// # Returns
    ///
    /// A new `ShotResult` instance containing the processed measurement results
    ///
    /// # Errors
    ///
    /// Returns an error if the `ByteMessage` cannot be parsed or doesn't contain valid measurement results
    pub fn from_byte_message(
        message: &ByteMessage,
        result_id_to_name: &HashMap<usize, String>,
    ) -> Result<Self, QueueError> {
        // Extract the measurement results from the ByteMessage
        let measurements = message.measurement_results_as_vec()?;

        let mut result = Self::default();

        // Process each measurement
        for (result_id, value) in measurements {
            // Get the name for this result_id, or use a default if not found
            let name = result_id_to_name
                .get(&result_id)
                .cloned()
                .unwrap_or_else(|| format!("result_{result_id}"));

            // Add the measurement to the results
            result.measurements.insert(name, value);
        }

        Ok(result)
    }
}

/// Represents the results of multiple shots (executions) of a quantum program.
///
/// This struct contains a vector of shots, where each shot is represented as a
/// mapping of register names to measurement outcomes as strings.
#[derive(Debug, Clone)]
pub struct ShotResults {
    pub shots: Vec<HashMap<String, String>>,
}

impl Default for ShotResults {
    fn default() -> Self {
        Self::new()
    }
}

impl ShotResults {
    /// Creates a new empty `ShotResults` instance.
    #[must_use]
    pub fn new() -> Self {
        Self { shots: Vec::new() }
    }

    /// Creates a `ShotResults` instance from a slice of `ShotResult` instances.
    ///
    /// This method processes each `ShotResult`, extracting measurements and formatting
    /// them appropriately for the `ShotResults` structure.
    ///
    /// # Parameters
    ///
    /// * `results` - A slice of `ShotResult` instances to process
    ///
    /// # Returns
    ///
    /// A new `ShotResults` instance containing the processed measurement results
    #[must_use]
    pub fn from_measurements(results: &[ShotResult]) -> Self {
        let mut shots = Vec::new();

        for shot in results {
            let mut processed_results: HashMap<String, String> = HashMap::new();
            let mut measurement_values = Vec::new();

            let mut keys: Vec<_> = shot.measurements.keys().collect();
            keys.sort();

            for key in &keys {
                if key.starts_with("measurement_") {
                    if let Some(&value) = shot.measurements.get(*key) {
                        measurement_values.push(value.to_string());
                    }
                } else if let Some(&value) = shot.measurements.get(*key) {
                    processed_results.insert((*key).to_string(), value.to_string());
                }
            }

            // If we have a combined result from the engine, use it
            if let Some(combined) = &shot.combined_result {
                processed_results.insert("result".to_string(), combined.clone());
            } else if !measurement_values.is_empty() {
                // Otherwise, use the concatenated measurement values
                processed_results.insert("result".to_string(), measurement_values.concat());
            }

            shots.push(processed_results);
        }

        Self { shots }
    }

    /// Create a `ShotResults` instance directly from a `ByteMessage` containing measurement results.
    ///
    /// This method extracts measurement results from a `ByteMessage` and creates a `ShotResults`
    /// instance with properly formatted results. It's more efficient than going through
    /// `ShotResult` instances and provides better context about the measurements.
    ///
    /// # Parameters
    ///
    /// * `message` - A `ByteMessage` containing measurement results
    pub fn from_byte_message(message: &ByteMessage) -> Result<Self, QueueError> {
        // Extract the measurement results from the ByteMessage
        let measurements = message.measurement_results_as_vec()?;

        let mut result = Self::new();

        // Process each measurement
        for (result_id, value) in measurements {
            // Get the name for this result_id, or use a default if not found
            let name = format!("result_{result_id}");

            // Add the measurement to the results
            result.shots[0].insert(name, value.to_string());
        }

        Ok(result)
    }

    /// Prints the `ShotResults` to stdout.
    pub fn print(&self) {
        println!("{self}");
    }
}

impl fmt::Display for ShotResults {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "[")?;

        for (i, shot) in self.shots.iter().enumerate() {
            write!(f, "  {{")?;

            // Only include the "result" key in the output
            if let Some(result) = shot.get("result") {
                write!(f, "\"result\": \"{result}\"")?;
            }

            if i < self.shots.len() - 1 {
                writeln!(f, "}},")?;
            } else {
                writeln!(f, "}}")?;
            }
        }

        write!(f, "]")
    }
}
