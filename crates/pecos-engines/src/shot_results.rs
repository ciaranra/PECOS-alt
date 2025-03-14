use crate::channels::byte_message::ByteMessage;
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

            if !measurement_values.is_empty() {
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
    /// * `result_id_to_name` - A mapping from `result_id` to a human-readable name
    ///
    /// # Returns
    ///
    /// A new `ShotResults` instance containing the processed measurement results
    ///
    /// # Errors
    ///
    /// Returns an error if the `ByteMessage` cannot be parsed or doesn't contain valid measurement results
    pub fn from_byte_message(
        message: &ByteMessage,
        result_id_to_name: &HashMap<usize, String>,
    ) -> Result<Self, QueueError> {
        use std::collections::HashMap;

        // Extract the measurement results from the ByteMessage
        let measurements = message.measurement_results_as_vec()?;

        // Create a single shot result (since a ByteMessage represents one shot)
        let mut processed_results: HashMap<String, String> = HashMap::new();

        // Process each measurement
        for (result_id, value) in &measurements {
            // Get the name for this result_id, or use a default if not found
            let name = result_id_to_name
                .get(result_id)
                .cloned()
                .unwrap_or_else(|| format!("result_{result_id}"));

            // Add the measurement to the results
            processed_results.insert(name, value.to_string());
        }

        // If we have measurements, also create a combined "result" entry
        if !measurements.is_empty() {
            // Sort by result_id for consistent ordering
            let mut sorted_measurements: Vec<_> = measurements.iter().collect();
            sorted_measurements.sort_by_key(|(id, _)| *id);

            // Create the combined result string
            let result_string: String = sorted_measurements
                .iter()
                .map(|(_, value)| value.to_string())
                .collect();

            processed_results.insert("result".to_string(), result_string);
        }

        // Create and return the ShotResults
        Ok(Self {
            shots: vec![processed_results],
        })
    }

    /// Create a `ShotResults` instance from multiple `ByteMessage` instances, each representing a shot.
    ///
    /// This method is useful for multi-shot simulations where each shot produces a `ByteMessage`
    /// with measurement results.
    ///
    /// # Parameters
    ///
    /// * `messages` - A slice of `ByteMessage` instances, each containing measurement results for one shot
    /// * `result_id_to_name` - A mapping from `result_id` to a human-readable name
    ///
    /// # Returns
    ///
    /// A new `ShotResults` instance containing the processed measurement results from all shots
    ///
    /// # Errors
    ///
    /// Returns an error if any `ByteMessage` cannot be parsed or doesn't contain valid measurement results
    pub fn from_byte_messages(
        messages: &[ByteMessage],
        result_id_to_name: &HashMap<usize, String>,
    ) -> Result<Self, QueueError> {
        let mut shots = Vec::with_capacity(messages.len());

        // Process each message (shot)
        for message in messages {
            // Extract and process the measurements for this shot
            let shot_result = Self::from_byte_message(message, result_id_to_name)?;

            // Add the processed results to our collection
            if let Some(shot) = shot_result.shots.first() {
                shots.push(shot.clone());
            }
        }

        Ok(Self { shots })
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
            // Get all keys and sort them for consistent output
            let mut keys: Vec<_> = shot.keys().collect();
            keys.sort();

            write!(f, "  {{")?;
            for (j, key) in keys.iter().enumerate() {
                write!(f, "\"{}\": \"{}\"", key, shot.get(*key).unwrap())?;
                if j < keys.len() - 1 {
                    write!(f, ", ")?;
                }
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
