use plugin_system::{
    CoProcessor, DrivingProcessor, DynCoProcessor, DynDrivingProcessor, ProcessingSystem,
    ProcessorStage, Runner,
};
use processors::std_processors::{ArrayProcessor, BatchSummer, NumberDoubler};
use serde_json::{json, Value};

// Shared test case runner for both versions
fn run_test_cases(
    system: &mut ProcessingSystem<
        impl DrivingProcessor<Value, Value> + Clone + 'static,
        impl CoProcessor + Clone + 'static,
        Value,
        Value,
    >,
) -> Result<(), Box<dyn std::error::Error>> {
    let test_cases = vec![
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9],     // Divisible by 3
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10], // One extra
        vec![1, 2, 3, 4, 5, 6, 7, 8],        // One short
        vec![1, 2],                          // Small batch
    ];

    for numbers in test_cases {
        println!("\nInput:  {:?}", numbers);
        let input = json!({"numbers": numbers});
        let result = system.process(input);

        let output = result
            .as_object()
            .and_then(|obj| obj.get("numbers"))
            .and_then(|arr| arr.as_array())
            .map(|arr| arr.iter().map(|v| v.as_i64().unwrap()).collect::<Vec<_>>())
            .unwrap_or_default();

        println!("Output: {:?}", output);
    }

    Ok(())
}

pub fn test_processing_system() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nProcessing System Demo (Direct Usage)");
    println!("===================================");

    // Create base processors directly
    let number_doubler = NumberDoubler;
    let batch_summer = BatchSummer::new();
    let array_processor = ArrayProcessor::new();

    // Create a processor stage combining BatchSummer with NumberDoubler
    let summer_stage: ProcessorStage<BatchSummer, NumberDoubler> =
        ProcessingSystem::new(batch_summer, number_doubler);

    // Create top level system using ArrayProcessor and our summer_stage
    let mut system = ProcessingSystem::new(array_processor, summer_stage);

    run_test_cases(&mut system)?;
    Ok(())
}

pub fn test_processing_system2(runner: &mut Runner) -> Result<(), Box<dyn std::error::Error>> {
    println!("\nProcessing System Demo (Plugin System)");
    println!("====================================");

    // First check for existence without holding the borrow
    let doubler_name = if runner.registry.get_coprocessor("NumberDoubler").is_some() {
        "NumberDoubler"
    } else {
        "PythonNumberDoubler"
    };

    let summer_name = if runner
        .registry
        .get_driving_processor("BatchSummer")
        .is_some()
    {
        "BatchSummer"
    } else {
        "PythonBatchAccumulator"
    };

    // Get each processor separately, releasing the borrow each time
    let number_doubler = {
        let processor = runner
            .registry
            .get_coprocessor(doubler_name)
            .expect("Couldn't find NumberDoubler plugin");
        DynCoProcessor::new(processor.clone_box())
    };

    let batch_summer = {
        let processor = runner
            .registry
            .get_driving_processor(summer_name)
            .expect("Couldn't find BatchSummer plugin");
        DynDrivingProcessor::new(processor.clone_box())
    };

    let array_processor = {
        let processor = runner
            .registry
            .get_driving_processor("ArrayProcessor")
            .expect("Couldn't find ArrayProcessor plugin");
        DynDrivingProcessor::new(processor.clone_box())
    };

    // Create a processor stage combining BatchSummer with NumberDoubler
    let summer_stage = ProcessingSystem::new(batch_summer, number_doubler);

    // Create top level system using ArrayProcessor and our summer_stage
    let mut system = ProcessingSystem::new(array_processor, summer_stage);

    run_test_cases(&mut system)?;
    Ok(())
}
