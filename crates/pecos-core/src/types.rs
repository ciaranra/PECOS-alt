use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Default)]
pub struct ShotResult {
    pub measurements: HashMap<String, u32>,
}

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
    #[must_use]
    pub fn new() -> Self {
        Self { shots: Vec::new() }
    }

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
