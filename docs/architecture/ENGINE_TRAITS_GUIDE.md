# PECOS Engine Traits Guide

This document provides a detailed guide to the engine traits in PECOS and how to implement custom engines.

## Core Engine Trait

The foundation of all engines is the `Engine` trait:

```rust
pub trait Engine: Clone + Send + Sync {
    type Input;
    type Output;
    
    fn process(&mut self, input: Self::Input) -> Result<Self::Output, PecosError>;
    fn reset(&mut self) -> Result<(), PecosError>;
}
```

**Key Points:**
- `Clone`: Required for parallel execution (each worker clones the engine)
- `Send + Sync`: Required for thread safety
- Associated types provide flexibility in input/output types

## Classical Engine Traits

### ClassicalEngine

The `ClassicalEngine` trait extends `Engine` with quantum-specific functionality:

```rust
pub trait ClassicalEngine: Engine<Input = (), Output = Shot> + DynClone + Send + Sync {
    /// Returns the number of qubits in the quantum program
    fn num_qubits(&self) -> usize;
    
    /// Generate quantum commands for the quantum engine
    fn generate_commands(&mut self) -> Result<ByteMessage, PecosError>;
    
    /// Process measurement results from the quantum engine
    fn handle_measurements(&mut self, message: ByteMessage) -> Result<(), PecosError>;
    
    /// Get the final results after all measurements
    fn get_results(&self) -> Result<Shot, PecosError>;
    
    /// Compile the quantum program (if applicable)
    fn compile(&self) -> Result<(), PecosError>;
    
    /// Set random seed for reproducibility
    fn set_seed(&mut self, seed: u64) -> Result<(), PecosError> {
        Ok(()) // Default: no-op
    }
    
    /// Reset the engine state
    fn reset(&mut self) -> Result<(), PecosError> {
        Ok(())
    }
    
    /// Type erasure support
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
```

### ControlEngine

The `ControlEngine` trait manages execution flow:

```rust
pub trait ControlEngine: Clone + Send + Sync {
    type Input;
    type Output;
    type EngineInput;
    type EngineOutput;
    
    /// Start processing with initial input
    fn start(&mut self, input: Self::Input) 
        -> Result<EngineStage<Self::EngineInput, Self::Output>, PecosError>;
    
    /// Continue processing with results from controlled engine
    fn continue_processing(&mut self, result: Self::EngineOutput)
        -> Result<EngineStage<Self::EngineInput, Self::Output>, PecosError>;
    
    /// Reset the engine state
    fn reset(&mut self) -> Result<(), PecosError>;
}

/// Control flow stages
pub enum EngineStage<I, O> {
    /// More processing needed, with input for controlled engine
    NeedsProcessing(I),
    /// Processing complete with final output
    Complete(O),
}
```

### ClassicalControlEngine

This trait combines both `ClassicalEngine` and `ControlEngine`:

```rust
pub trait ClassicalControlEngine: 
    ClassicalEngine + 
    ControlEngine<Input = (), Output = Shot, EngineInput = ByteMessage, EngineOutput = ByteMessage> 
{}

// Blanket implementation for all types that implement both traits
impl<T> ClassicalControlEngine for T 
where
    T: ClassicalEngine + ControlEngine<...>
{}
```

## Implementing a Classical Engine

Here's a template for implementing a custom classical engine:

```rust
#[derive(Clone)]
pub struct MyCustomEngine {
    program: MyProgram,
    state: ExecutionState,
    results: HashMap<String, Vec<i64>>,
}

// Implement ClassicalEngine
impl ClassicalEngine for MyCustomEngine {
    fn num_qubits(&self) -> usize {
        self.program.count_qubits()
    }
    
    fn generate_commands(&mut self) -> Result<ByteMessage, PecosError> {
        // Generate next batch of quantum commands
        let mut builder = ByteMessage::builder();
        
        while let Some(instruction) = self.state.next_instruction() {
            match instruction {
                Instruction::Gate(gate) => {
                    // Add gate to ByteMessage
                    builder = builder.add_gate(...);
                }
                Instruction::Measure(qubit, reg) => {
                    // Add measurement
                    builder = builder.add_measurement(...);
                    // May need to wait for result
                    break;
                }
            }
        }
        
        Ok(builder.build())
    }
    
    fn handle_measurements(&mut self, message: ByteMessage) -> Result<(), PecosError> {
        // Process measurement results
        let outcomes = message.outcomes()?;
        
        // Update internal state based on measurements
        for (idx, outcome) in outcomes.iter().enumerate() {
            self.state.set_measurement_result(idx, *outcome);
        }
        
        Ok(())
    }
    
    fn get_results(&self) -> Result<Shot, PecosError> {
        // Convert internal results to Shot format
        let mut shot = Shot::default();
        
        for (name, values) in &self.results {
            shot.data.insert(name.clone(), Data::I64(values.last().copied().unwrap_or(0)));
        }
        
        Ok(shot)
    }
    
    fn compile(&self) -> Result<(), PecosError> {
        // Validate/compile the program if needed
        self.program.validate()?;
        Ok(())
    }
    
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// Implement ControlEngine
impl ControlEngine for MyCustomEngine {
    type Input = ();
    type Output = Shot;
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;
    
    fn start(&mut self, _input: ()) -> Result<EngineStage<ByteMessage, Shot>, PecosError> {
        // Reset state for new execution
        self.state.reset();
        
        // Generate first commands
        match self.generate_commands()? {
            cmd if cmd.is_empty() => {
                // No commands, we're done
                Ok(EngineStage::Complete(self.get_results()?))
            }
            cmd => {
                // Send commands for processing
                Ok(EngineStage::NeedsProcessing(cmd))
            }
        }
    }
    
    fn continue_processing(
        &mut self, 
        measurements: ByteMessage
    ) -> Result<EngineStage<ByteMessage, Shot>, PecosError> {
        // Handle measurements
        self.handle_measurements(measurements)?;
        
        // Generate next commands or complete
        match self.generate_commands()? {
            cmd if cmd.is_empty() => {
                Ok(EngineStage::Complete(self.get_results()?))
            }
            cmd => {
                Ok(EngineStage::NeedsProcessing(cmd))
            }
        }
    }
    
    fn reset(&mut self) -> Result<(), PecosError> {
        self.state.reset();
        self.results.clear();
        Ok(())
    }
}

// Implement base Engine trait
impl Engine for MyCustomEngine {
    type Input = ();
    type Output = Shot;
    
    fn process(&mut self, input: Self::Input) -> Result<Self::Output, PecosError> {
        // Simple implementation using ControlEngine
        let mut stage = self.start(input)?;
        
        while let EngineStage::NeedsProcessing(commands) = stage {
            // In standalone mode, we'd need a quantum engine here
            // For now, return empty measurements
            let measurements = ByteMessage::builder().build();
            stage = self.continue_processing(measurements)?;
        }
        
        match stage {
            EngineStage::Complete(output) => Ok(output),
            _ => unreachable!(),
        }
    }
    
    fn reset(&mut self) -> Result<(), PecosError> {
        ControlEngine::reset(self)
    }
}
```

## Quantum Engine Implementation

Quantum engines are simpler, processing `ByteMessage` in and out:

```rust
pub struct MyQuantumEngine {
    state: QuantumState,
    num_qubits: usize,
}

impl Engine for MyQuantumEngine {
    type Input = ByteMessage;
    type Output = ByteMessage;
    
    fn process(&mut self, input: Self::Input) -> Result<Self::Output, PecosError> {
        let mut results = ByteMessage::builder();
        
        // Process quantum operations
        for op in input.quantum_ops()? {
            match op {
                QuantumOp::Gate { gate, qubits } => {
                    self.apply_gate(gate, qubits)?;
                }
                QuantumOp::Measure { qubit, basis } => {
                    let outcome = self.measure(qubit, basis)?;
                    results = results.add_measurement_outcome(outcome);
                }
            }
        }
        
        Ok(results.build())
    }
    
    fn reset(&mut self) -> Result<(), PecosError> {
        self.state = QuantumState::new(self.num_qubits);
        Ok(())
    }
}

impl QuantumEngine for MyQuantumEngine {
    fn num_qubits(&self) -> usize {
        self.num_qubits
    }
    
    // Other trait methods...
}
```

## Using Your Engine

### With HybridEngine

```rust
// Create engines
let classical = MyCustomEngine::new(program);
let quantum = StateVecEngine::new(num_qubits);
let noise = PassThroughNoiseModel;

// Combine into quantum system
let quantum_system = EngineSystem::new(noise, quantum);

// Combine into hybrid engine
let hybrid = HybridEngine::new(classical, quantum_system);

// Run single shot
let result = hybrid.process(())?;
```

### With MonteCarloEngine

```rust
// Use with Monte Carlo engine for parallel execution
let results = MonteCarloEngine::run_with_noise_model(
    Box::new(classical_engine),
    Box::new(noise_model),
    shots,
    workers,
    seed,
)?;
```

### With Builder API

Create a builder API for user convenience:

```rust
pub fn my_custom_sim(program: MyProgram) -> MyCustomSimBuilder {
    MyCustomSimBuilder::new(program)
}

pub struct MyCustomSimBuilder {
    program: MyProgram,
    config: SimConfig,
}

impl MyCustomSimBuilder {
    pub fn seed(mut self, seed: u64) -> Self {
        self.config.seed = Some(seed);
        self
    }
    
    pub fn run(self, shots: usize) -> Result<HashMap<String, Vec<i64>>, PecosError> {
        let engine = MyCustomEngine::new(self.program);
        
        let results = MonteCarloEngine::run_with_noise_model(
            Box::new(engine),
            self.config.noise_model,
            shots,
            self.config.workers,
            self.config.seed,
        )?;
        
        // Convert to columnar format
        Ok(convert_to_columnar(results))
    }
}
```

## Best Practices

### 1. State Management
- Keep execution state separate from program definition
- Make state resettable for multiple runs
- Track measurement results incrementally

### 2. Error Handling
- Use descriptive error messages
- Validate inputs early
- Handle edge cases gracefully

### 3. Performance
- Minimize allocations in hot paths
- Pre-compile/validate programs when possible
- Use efficient data structures

### 4. Testing
- Test both `ClassicalEngine` and `ControlEngine` interfaces
- Verify `Clone` implementation works correctly
- Test with various quantum engines
- Ensure deterministic behavior with seeds

### 5. Documentation
- Document quantum gate semantics
- Explain measurement behavior
- Provide usage examples
- Note any limitations