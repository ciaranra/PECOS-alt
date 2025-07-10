# PECOS Engine System Architecture

## Overview

The PECOS engine system provides a flexible, composable architecture for quantum simulation that separates concerns between classical control flow, quantum execution, noise modeling, and orchestration.

## Architecture Layers

```
┌─────────────────────────────────────────────────────────────┐
│                    User API Layer                           │
│         qasm_sim()    llvm_sim()    guppy_sim()            │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│                 Orchestration Layer                         │
│                   MonteCarloEngine                          │
│  • Parallel execution across workers                        │
│  • Shot distribution and result aggregation                 │
│  • Seed management for reproducibility                      │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│                  Coordination Layer                         │
│                    HybridEngine                             │
│  • Combines ClassicalEngine + QuantumSystem                 │
│  • Manages control flow between components                  │
└────────┬────────────────────────────┬───────────────────────┘
         │                            │
┌────────▼──────────┐        ┌───────▼───────────────────────┐
│ Classical Engine  │        │      Quantum System           │
│                   │        │                               │
│ • QasmEngine      │        │  ┌─────────────────────┐     │
│ • LlvmEngine      │        │  │   Noise Model       │     │
│ • PhirEngine      │        │  │ • PassThrough       │     │
│                   │        │  │ • Depolarizing      │     │
│ Generates quantum │        │  │ • BiasedDepolarizing│     │
│ commands and      │        │  └──────────┬──────────┘     │
│ handles results   │        │             │                 │
└───────────────────┘        │  ┌──────────▼──────────┐     │
                             │  │  Quantum Engine     │     │
                             │  │ • StateVecEngine    │     │
                             │  │ • SparseStabEngine  │     │
                             │  └─────────────────────┘     │
                             └───────────────────────────────┘
```

## Core Components

### 1. Classical Engines

Classical engines implement both `ClassicalEngine` and `ControlEngine` traits:

```rust
pub trait ClassicalEngine: Clone + Send + Sync {
    fn num_qubits(&self) -> usize;
    fn generate_commands(&mut self) -> Result<ByteMessage, PecosError>;
    fn handle_measurements(&mut self, message: ByteMessage) -> Result<(), PecosError>;
    fn get_results(&self) -> Result<Shot, PecosError>;
    fn compile(&self) -> Result<(), PecosError>;
    fn reset(&mut self) -> Result<(), PecosError>;
}

pub trait ControlEngine: Clone + Send + Sync {
    type Input;
    type Output;
    type EngineInput;
    type EngineOutput;
    
    fn start(&mut self, input: Self::Input) 
        -> Result<EngineStage<Self::EngineInput, Self::Output>, PecosError>;
    
    fn continue_processing(&mut self, result: Self::EngineOutput)
        -> Result<EngineStage<Self::EngineInput, Self::Output>, PecosError>;
}
```

**Implementations:**
- **QasmEngine**: Executes OpenQASM 2.0 programs
- **LlvmEngine**: Executes LLVM IR quantum programs
- **PhirEngine**: Executes PHIR (PECOS HIR) programs

### 2. Quantum Engines

Quantum engines execute quantum operations and return measurements:

```rust
pub trait QuantumEngine: Engine<Input = ByteMessage, Output = ByteMessage> + Clone {
    fn get_state(&self) -> Result<Vec<Complex64>, PecosError>;
    fn num_qubits(&self) -> usize;
    // Additional methods...
}
```

**Implementations:**
- **StateVecEngine**: Full state vector simulation
- **SparseStabEngine**: Efficient stabilizer simulation for Clifford circuits

### 3. Noise Models

Noise models transform quantum operations before execution:

```rust
pub trait NoiseModel: Engine<Input = ByteMessage, Output = ByteMessage> + Clone {
    fn set_seed(&mut self, seed: u64);
}
```

**Implementations:**
- **PassThroughNoiseModel**: No noise (ideal simulation)
- **DepolarizingNoiseModel**: Standard depolarizing noise
- **BiasedDepolarizingNoiseModel**: Biased depolarizing noise
- **GeneralNoiseModel**: Configurable noise model

### 4. Engine System Pattern

The `EngineSystem` trait enables composition:

```rust
pub trait EngineSystem<T, K>: Engine<Input = T::Input, Output = K::Output>
where
    T: ControlEngine<EngineInput = K::Input, EngineOutput = K::Output>,
    K: Engine,
{
    fn controller(&self) -> &T;
    fn controller_mut(&mut self) -> &mut T;
    fn controlled(&self) -> &K;
    fn controlled_mut(&mut self) -> &mut K;
}
```

This pattern creates:
- **HybridEngine**: ClassicalControlEngine + QuantumSystem
- **QuantumSystem**: NoiseModel + QuantumEngine

## Data Flow

### 1. Command Generation
```
User Input → Classical Engine → ByteMessage (quantum commands)
```

### 2. Quantum Execution
```
ByteMessage → Noise Model → Transformed ByteMessage → Quantum Engine
```

### 3. Measurement Flow
```
Quantum Engine → ByteMessage (measurements) → Classical Engine → Shot Results
```

### 4. Control Flow
```rust
enum EngineStage<I, O> {
    NeedsProcessing(I),  // More quantum operations to execute
    Complete(O),         // Computation finished
}
```

## Parallelization with MonteCarloEngine

The `MonteCarloEngine` orchestrates parallel execution:

1. **Template Creation**: Creates a template `HybridEngine`
2. **Worker Spawning**: Spawns worker threads (default: CPU cores - 1)
3. **Seed Distribution**: Each worker gets a unique seed derived from base seed
4. **Shot Distribution**: Shots are distributed across workers
5. **Result Aggregation**: Results collected in deterministic order

```rust
MonteCarloEngine::run_with_noise_model(
    classical_engine,  // Box<dyn ClassicalControlEngine>
    noise_model,       // Box<dyn NoiseModel>
    shots,             // Number of shots
    workers,           // Number of parallel workers
    seed,              // Optional seed
)
```

## User API Design

### Builder Pattern APIs

All user-facing APIs (`qasm_sim`, `llvm_sim`, `guppy_sim`) follow a consistent builder pattern:

```rust
// Example: llvm_sim
let results = llvm_sim(llvm_ir)
    .seed(42)                          // Reproducibility
    .workers(8)                        // Parallelization
    .with_depolarizing_noise(0.01)     // Noise model
    .with_state_vector_engine()        // Quantum engine
    .run(1000)?;                       // Execute shots

// Build once, run many
let mut sim = llvm_sim(llvm_ir).build()?;
let results1 = sim.run(100)?;
let results2 = sim.run(1000)?;
```

### Result Format

Results are returned in columnar format:
```rust
HashMap<String, Vec<i64>>  // {"register_name": [shot1, shot2, ...]}
```

## Key Design Principles

### 1. Separation of Concerns
- Classical engines handle control flow
- Quantum engines handle quantum state
- Noise models handle error injection
- Orchestrators handle parallelization

### 2. Composability
- Engines combine via `EngineSystem` trait
- Any classical engine works with any quantum engine
- Noise models are pluggable

### 3. Type Safety
- Strong typing with associated types
- Compile-time verification of engine compatibility
- Clear input/output contracts

### 4. Performance
- Engines are `Clone` for parallel execution
- Efficient `ByteMessage` for data transfer
- Minimal allocations in hot paths

### 5. Extensibility
- New classical engines (e.g., for new languages)
- New quantum engines (e.g., tensor network)
- New noise models (e.g., coherent errors)

## Example: Complete Flow

```rust
// 1. User calls high-level API
let results = qasm_sim(qasm_code)
    .seed(42)
    .workers(4)
    .with_depolarizing_noise(0.01)
    .run(1000)?;

// 2. Under the hood:
// - Creates QasmEngine (ClassicalEngine)
// - Creates DepolarizingNoiseModel
// - Creates StateVecEngine (QuantumEngine)
// - Combines into QuantumSystem (Noise + Quantum)
// - Combines into HybridEngine (Classical + QuantumSystem)
// - MonteCarloEngine clones HybridEngine for each worker
// - Workers execute shots in parallel
// - Results aggregated and returned
```

## Testing Strategy

### Unit Tests
- Individual engine functionality
- Trait implementations
- Data serialization

### Integration Tests
- Engine composition
- End-to-end simulation
- Noise model effects

### Comparison Tests
- Direct engine vs API equivalence
- Cross-language consistency
- Numerical accuracy

## Future Extensions

### Potential Additions
1. **Tensor Network Engine**: For larger circuits
2. **GPU Acceleration**: For StateVecEngine
3. **Distributed Execution**: Cross-machine parallelization
4. **Real Hardware Backends**: Interface with quantum hardware
5. **Circuit Optimization**: Pre-execution optimization passes

### API Evolution
- Streaming results for large simulations
- Progress callbacks
- Resource estimation
- Error mitigation strategies