use proto_bytemessage::message::{BatchBuilder, MessageBatch, MessageType};
use proto_bytemessage::process::{CoProcessor, DrivingProcessor, ProcessingStage};
use serde_json::{Value, json};

/// Bottom level processor that just doubles numbers
#[allow(dead_code)]
pub(crate) struct NumberDoubler;

impl CoProcessor for NumberDoubler {
    fn process(&mut self, input: MessageBatch) -> MessageBatch {
        let mut builder = BatchBuilder::new();
        for (_header, payload) in input.iter() {
            if let Ok(number) = bytemuck::try_from_bytes::<u32>(payload) {
                let doubled = number * 2;
                builder.add(MessageType::Example, bytemuck::bytes_of(&doubled));
            }
        }
        builder.build()
    }
}

/// Middle level processor that accumulates sums in groups of 3
pub(crate) struct BatchSummer {
    current_batch: Vec<u32>,
}

impl BatchSummer {
    pub(crate) fn new() -> Self {
        Self {
            current_batch: Vec::new(),
        }
    }
}

impl DrivingProcessor<MessageBatch, MessageBatch> for BatchSummer {
    fn start(&mut self, input: MessageBatch) -> ProcessingStage<MessageBatch, MessageBatch> {
        self.current_batch.clear();
        ProcessingStage::NeedsCoprocessing(input)
    }

    fn continue_processing(
        &mut self,
        results: MessageBatch,
    ) -> ProcessingStage<MessageBatch, MessageBatch> {
        // First, add any new numbers to our batch
        let mut is_last_batch = true; // Assume it's the last unless we find numbers
        for (_header, payload) in results.iter() {
            if let Ok(number) = bytemuck::try_from_bytes::<u32>(payload) {
                is_last_batch = false;
                self.current_batch.push(*number);
            }
        }

        // If it's the last batch and we have no numbers, complete with empty batch
        if is_last_batch && self.current_batch.is_empty() {
            return ProcessingStage::Complete(BatchBuilder::new().build());
        }

        // Process what we have
        let mut builder = BatchBuilder::new();

        if self.current_batch.len() >= 3 {
            // We have at least one complete group
            let sum: u32 = self.current_batch.drain(..3).sum();
            builder.add(MessageType::Example, bytemuck::bytes_of(&sum));
            ProcessingStage::Complete(builder.build())
        } else if is_last_batch {
            // Final batch and we have some remaining numbers
            let sum: u32 = self.current_batch.drain(..).sum();
            builder.add(MessageType::Example, bytemuck::bytes_of(&sum));
            ProcessingStage::Complete(builder.build())
        } else {
            // Need more numbers for a complete group
            ProcessingStage::NeedsCoprocessing(BatchBuilder::new().build())
        }
    }
}

/// Top level processor that processes JSON arrays
pub(crate) struct ArrayProcessor {
    numbers: Vec<u32>,
    position: usize,
    results: Vec<u32>,
}

impl ArrayProcessor {
    pub(crate) fn new() -> Self {
        Self {
            numbers: Vec::new(),
            position: 0,
            results: Vec::new(),
        }
    }

    fn next_batch(&mut self) -> Option<MessageBatch> {
        if self.position >= self.numbers.len() {
            if self.position == self.numbers.len() {
                // Send one final empty batch to signal end
                self.position += 1;
                Some(BatchBuilder::new().build())
            } else {
                None
            }
        } else {
            let mut builder = BatchBuilder::new();
            // Changed to process 3 numbers at a time
            let end_pos = (self.position + 3).min(self.numbers.len());

            for &num in &self.numbers[self.position..end_pos] {
                builder.add(MessageType::Example, bytemuck::bytes_of(&num));
            }

            self.position = end_pos;
            Some(builder.build())
        }
    }
}

impl DrivingProcessor<Value, Value> for ArrayProcessor {
    fn start(&mut self, input: Value) -> ProcessingStage<MessageBatch, Value> {
        // Extract numbers from JSON array
        self.numbers = input
            .as_array()
            .expect("Expected JSON array")
            .iter()
            .map(|v| {
                u32::try_from(v.as_u64().expect("Number conversion to u64 failed"))
                    .expect("Number conversion failed")
            })
            .collect();
        self.position = 0;
        self.results.clear();

        match self.next_batch() {
            Some(batch) => ProcessingStage::NeedsCoprocessing(batch),
            None => ProcessingStage::Complete(json!([])),
        }
    }

    fn continue_processing(
        &mut self,
        results: MessageBatch,
    ) -> ProcessingStage<MessageBatch, Value> {
        // Store any results
        for (_header, payload) in results.iter() {
            if let Ok(sum) = bytemuck::try_from_bytes::<u32>(payload) {
                self.results.push(*sum);
            }
        }

        // Process next batch if available
        if let Some(batch) = self.next_batch() {
            ProcessingStage::NeedsCoprocessing(batch)
        } else {
            // Convert final results to JSON
            let result = Value::Array(self.results.iter().map(|&n| json!(n)).collect());
            ProcessingStage::Complete(result)
        }
    }
}
