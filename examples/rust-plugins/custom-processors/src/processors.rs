use core::StructMetadata;
use processors::process::CoProcessor;
use serde_json::{json, Value};
use std::fmt::Debug;

/// A processor that multiplies numbers by a configurable factor
#[derive(Debug, Clone, StructMetadata)]
pub struct NumberMultiplier {
    factor: i64,
}

impl NumberMultiplier {
    pub fn new(factor: i64) -> Self {
        Self { factor }
    }
}

impl CoProcessor for NumberMultiplier {
    fn process(&mut self, input: Value) -> Value {
        let numbers = input["numbers"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|v| v.as_i64())
            .map(|n| n * self.factor)
            .collect::<Vec<_>>();

        json!({ "numbers": numbers })
    }

    fn clone_box(&self) -> Box<dyn CoProcessor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_number_multiplier() {
        let mut multiplier = NumberMultiplier::new(3);
        let input = json!({
            "numbers": [1, 2, 3, 4, 5]
        });

        let result = multiplier.process(input);

        let expected = json!({
            "numbers": [3, 6, 9, 12, 15]
        });

        assert_eq!(result, expected);
    }

    #[test]
    fn test_empty_input() {
        let mut multiplier = NumberMultiplier::new(3);
        let input = json!({
            "numbers": []
        });

        let result = multiplier.process(input);

        let expected = json!({
            "numbers": []
        });

        assert_eq!(result, expected);
    }

    #[test]
    fn test_missing_numbers() {
        let mut multiplier = NumberMultiplier::new(3);
        let input = json!({});

        let result = multiplier.process(input);

        let expected = json!({
            "numbers": []
        });

        assert_eq!(result, expected);
    }
}
