mod example_processors;

use crate::example_processors::{ArrayProcessor, BatchSummer, NumberDoubler};
use proto_bytemessage::process::{ProcessingSystem, ProcessorStage};
use serde_json::json;

fn main() {
    let number_doubler = NumberDoubler;
    let batch_summer = BatchSummer::new();
    let array_processor = ArrayProcessor::new();

    let summer_stage: ProcessorStage<BatchSummer, NumberDoubler> =
        ProcessorStage::new(batch_summer, number_doubler);

    let mut system = ProcessingSystem::new(array_processor, summer_stage);

    let test_cases = [
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9],     // Divisible by 3
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10], // One extra
        vec![1, 2, 3, 4, 5, 6, 7, 8],        // One short
        vec![1, 2],
    ];

    for numbers in &test_cases {
        let input = json!(numbers);
        let result = system.process(input);
        let output: Vec<u64> = result
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect();

        println!("Input:  {numbers:?}");
        println!("Output: {output:?}\n");
    }
}
