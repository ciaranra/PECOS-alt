#[cfg(test)]
mod compile_time_tests {
    use crate::{run_sim, ClassicalEngine, NoiseModel, QuantumEngine};
    use crate::simulation_builder::SimulationBuilder;

    /// This test exists to ensure run_sim parameters are in correct order at compile time
    #[test]
    fn test_run_sim_parameter_order() {
        // This function will fail to compile if parameter types change
        fn _check_signature(
            _classical: Box<dyn ClassicalEngine>,
            _shots: usize,
            _seed: Option<u64>,
            _workers: Option<usize>,
            _noise: Option<Box<dyn NoiseModel>>,
            _quantum: Option<Box<dyn QuantumEngine>>,
        ) {
            // Just a type check, doesn't need to do anything
        }
        
        // This ensures the function signature hasn't changed
        let _ = run_sim as fn(
            Box<dyn ClassicalEngine>,
            usize,
            Option<u64>,
            Option<usize>,
            Option<Box<dyn NoiseModel>>,
            Option<Box<dyn QuantumEngine>>,
        ) -> Result<_, _>;
    }

    /// Test that builder pattern compiles correctly
    #[test]
    fn test_builder_pattern_compiles() {
        // This test just needs to compile, not run
        if false {
            let _builder = SimulationBuilder::new()
                .shots(100)
                .seed(42)
                .workers(4);
            // Type checking only
        }
    }
}

/// Macro to generate compile-time checks for function calls
#[macro_export]
macro_rules! assert_param_types {
    ($func:ident($($param:expr),*) => $expected:ty) => {
        {
            let _: $expected = $func($($param),*);
        }
    };
}