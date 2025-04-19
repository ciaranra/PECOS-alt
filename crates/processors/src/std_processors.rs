use crate::process::{CoProcessor, DrivingProcessor, ProcessingStage};
use pecos_core::StructMetadata;
use serde_json::{Value, json};

/// Doubles each number in a batch
#[derive(Debug, Clone, StructMetadata)]
pub struct NumberDoubler;

impl CoProcessor for NumberDoubler {
    fn process(&mut self, input: Value) -> Value {
        let numbers = input["numbers"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|v| v.as_i64())
            .map(|n| n * 2)
            .collect::<Vec<_>>();

        json!({ "numbers": numbers })
    }

    fn clone_box(&self) -> Box<dyn CoProcessor> {
        Box::new(NumberDoubler)
    }
}

/// Sums numbers in batches of three
#[derive(Debug, Clone, StructMetadata)]
pub struct BatchSummer {
    current_batch: Vec<i64>,
}

impl Default for BatchSummer {
    fn default() -> Self {
        Self::new()
    }
}

impl BatchSummer {
    pub fn new() -> Self {
        Self {
            current_batch: Vec::new(),
        }
    }
}

impl DrivingProcessor<Value, Value> for BatchSummer {
    fn start(&mut self, input: Value) -> ProcessingStage<Value, Value> {
        self.current_batch.clear();
        ProcessingStage::NeedsCoprocessing(input)
    }

    fn continue_processing(&mut self, results: Value) -> ProcessingStage<Value, Value> {
        // Add new numbers to our batch
        let empty_vec = Vec::new();
        let numbers: Vec<i64> = results["numbers"]
            .as_array()
            .unwrap_or(&empty_vec)
            .iter()
            .filter_map(|v| v.as_i64())
            .collect();

        let got_new_numbers = !numbers.is_empty();

        self.current_batch.extend(numbers);

        // Process complete groups of 3 and handle any remainder
        let mut results = Vec::new();
        let chunk_size = 3;

        // Process as many complete chunks as possible
        for chunk in self.current_batch.chunks(chunk_size) {
            if chunk.len() == chunk_size {
                let sum = chunk.iter().sum::<i64>();
                results.push(sum);
            }
        }

        // Find how many complete chunks we processed
        let processed = (self.current_batch.len() / chunk_size) * chunk_size;

        // Keep any remaining numbers for the next batch
        self.current_batch = if processed < self.current_batch.len() {
            self.current_batch[processed..].to_vec()
        } else {
            Vec::new()
        };

        // If we have complete results or no new numbers coming, we're done
        if !results.is_empty() || (!got_new_numbers && !self.current_batch.is_empty()) {
            // If we have any remaining numbers, process them too
            if !self.current_batch.is_empty() {
                let final_sum = self.current_batch.iter().sum::<i64>();
                results.push(final_sum);
                self.current_batch.clear();
            }
            ProcessingStage::Complete(json!({ "numbers": results }))
        } else {
            ProcessingStage::NeedsCoprocessing(json!({ "numbers": [] }))
        }
    }

    fn clone_box(&self) -> Box<dyn DrivingProcessor<Value, Value>> {
        Box::new(BatchSummer {
            current_batch: self.current_batch.clone(),
        })
    }
}

/// Processes arrays of numbers in batches
#[derive(Debug, Clone, StructMetadata)]
pub struct ArrayProcessor {
    numbers: Vec<i64>,
    position: usize,
}

impl Default for ArrayProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl ArrayProcessor {
    pub fn new() -> Self {
        Self {
            numbers: Vec::new(),
            position: 0,
        }
    }
}

impl DrivingProcessor<Value, Value> for ArrayProcessor {
    fn start(&mut self, input: Value) -> ProcessingStage<Value, Value> {
        // Extract numbers from input array
        self.numbers = input["numbers"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|v| v.as_i64())
            .collect();

        self.position = 0;

        if self.numbers.is_empty() {
            ProcessingStage::Complete(json!({ "numbers": [] }))
        } else {
            // Send first batch
            let batch = &self.numbers[..std::cmp::min(3, self.numbers.len())];
            self.position = batch.len();
            ProcessingStage::NeedsCoprocessing(json!({ "numbers": batch }))
        }
    }

    fn continue_processing(&mut self, results: Value) -> ProcessingStage<Value, Value> {
        if self.position >= self.numbers.len() {
            // We're done - return the final results
            ProcessingStage::Complete(results)
        } else {
            // Send next batch
            let end = std::cmp::min(self.position + 3, self.numbers.len());
            let batch = &self.numbers[self.position..end];
            self.position = end;
            ProcessingStage::NeedsCoprocessing(json!({ "numbers": batch }))
        }
    }

    fn clone_box(&self) -> Box<dyn DrivingProcessor<Value, Value>> {
        Box::new(ArrayProcessor {
            numbers: self.numbers.clone(),
            position: self.position,
        })
    }
}
