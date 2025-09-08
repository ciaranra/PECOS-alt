# Unified Simulation API Design

## Overview

This document describes the design for a unified simulation API in PECOS that provides:
1. Engine-specific APIs for full control and type safety
2. A unified API for ease of use and engine flexibility
3. Clear separation between program data and execution engines
4. Zero-dependency program types shared across crates
5. Consistent API patterns across Python and Rust

## Core Principles

1. **Decoupling**: Program types (QASM, LLVM, HUGR) are independent of execution engines
2. **Progressive Disclosure**: Simple cases are simple, complex cases are possible
3. **Type Safety**: Engine-specific APIs preserve type information
4. **Zero-Copy**: In-memory objects are passed efficiently without serialization
5. **Extensibility**: Easy to add new program types and engines
6. **Automatic Conversions**: Engines accept both shared and engine-specific program types
7. **Consistency**: All builders/configs follow the same pattern in each language
8. **Idiomatic**: APIs follow language-specific conventions while maintaining conceptual consistency
9. **Lazy Evaluation**: Nothing is built until `.build()` or `.run()` is called
10. **Builder Inputs**: Methods accept only basic data types or builder objects, not mixed types

## Future Quantum Backend Sizing

Currently, quantum backends use static sizing where `.qubits(n)` sets both the capacity and maximum limit. In the future, as we introduce dynamically-sized quantum backends:

- **Current**: `.qubits(20)` → exactly 20 qubits (both capacity and limit)
- **Future**: `.qubits(20)` → initial/working capacity, `.max_qubits(100)` → hard upper limit

This evolution allows:
- Static backends: `.qubits(n)` continues to mean exact size
- Dynamic backends: `.qubits(n)` becomes initial capacity, can grow up to `.max_qubits(limit)`
- Backwards compatibility: existing code continues working unchanged

**Note**: For consistency, both quantum engine builders and classical engines (like LLVM) now use `.qubits()` to set their allocation limits. This provides a unified interface where `.qubits(n)` always means "allocate exactly n qubits" or "allow allocation up to n qubits" depending on the engine type.

## API Design Patterns

### Namespace Organization

Both Python and Rust organize functionality into logical namespaces for better discoverability and organization.

#### Python Namespaces

```python
from pecos_rslib import classical, quantum, noise, programs

# Clear, organized access
classical.qasm()            # Classical control engines
quantum.sparse_stabilizer() # Quantum backends
noise.depolarizing()        # Noise models
programs.QasmProgram        # Program types
```

#### Rust Namespaces (Proposed)

```rust
use pecos::{classical, quantum, noise, programs};

// Same organization as Python
classical::qasm()            // Classical control engines
quantum::sparse_stabilizer() // Quantum backends
noise::depolarizing()        // Noise models
programs::QasmProgram        // Program types
```

### Builder Pattern and Lazy Evaluation

#### Key Methods

The simulation builder provides these key configuration methods:

| Method | Purpose | Example |
|--------|---------|---------|
| `.quantum(builder)` | Set quantum simulator/engine | `.quantum(quantum.sparse_stab())` |
| `.qubits(n)` | Set number of qubits | `.qubits(20)` |
| `.noise(builder)` | Set noise model | `.noise(noise.depolarizing())` |
| `.seed(n)` | Set random seed | `.seed(42)` |
| `.workers(n)` | Set worker threads | `.workers(4)` |

#### Input Types

Methods accept only two types of inputs for consistency:

1. **Basic Data Types**: For simple parameters
   - `.seed(42)` - accepts `u64`/`int`
   - `.workers(4)` - accepts `usize`/`int`
   - `.max_qubits(100)` - accepts `usize`/`int`

2. **Builder Objects**: For complex configuration
   - `.quantum(sparse_stabilizer())` - accepts builder
   - `.noise(depolarizing().with_p1(0.01))` - accepts builder
   - `.program(QasmProgram::from_string("..."))` - accepts program object

**NOT Accepted**:
- Enums (use builders instead)
- Mixed types
- Pre-built engines or models

#### Lazy Evaluation Lifecycle

```rust
// Phase 1: Configuration (everything is builders/POD)
let sim_builder = sim_builder()
    .classical(engine)
    .seed(42)                    // Stores u64
    .quantum(sparse_stab()) // Stores builder
    .noise(depolarizing());        // Stores builder

// Phase 2a: Build once, run many times
let sim = sim_builder.build()?;  // Constructs all engines/models NOW
sim.run(100)?;   // Uses pre-built objects
sim.run(1000)?;  // Reuses same objects
sim.run(10000)?; // Efficient reuse

// Phase 2b: Or one-shot (build implicitly)
let results = sim_builder.run(1000)?; // Calls build() internally
```

**Key Points**:
- `SimBuilder` stores only configuration and builder objects
- `Simulation` stores built engines and models
- `.build()` transforms builders → built objects (expensive, done once)
- `.run()` uses built objects (cheap, can be done many times)
- `.run()` on builder calls `.build()` implicitly for convenience

### Consistent Builder/Config Pattern

Both languages provide the same API through free functions as the primary interface, with direct instantiation available as an alternative.

#### Primary API: Free Functions (Recommended)

Both Rust and Python use identical free function APIs:

```rust
// Rust
sim_builder()
    .classical(qasm_engine()
        .program(QasmProgram::from_string(qasm)))
    .quantum(sparse_stab())
    .noise(depolarizing_noise().with_p1_probability(0.01))
    .run(1000)?;
```

```python
# Python - identical to Rust!
(sim_builder()
    .classical(qasm_engine()
        .program(QasmProgram.from_string(qasm)))
    .quantum(sparse_stab())
    .noise(depolarizing_noise().with_p1_probability(0.01))
    .run(1000))
```

#### Alternative API: Direct Instantiation

For users who prefer explicit type construction:

##### Rust (with Builder suffix - idiomatic)
```rust
SimBuilder::new()
    .classical(QasmEngineBuilder::new()
        .program(QasmProgram::from_string(qasm)))
    .quantum(SparseStabBuilder::new())
    .noise(DepolarizingNoiseBuilder::new().with_p1_probability(0.01))
    .run(1000)?;
```

##### Python (without Builder suffix - concise)
```python
(SimBuilder()
    .classical(QasmEngine()
        .program(QasmProgram.from_string(qasm)))
    .quantum(SparseStab())
    .noise(DepolarizingNoise().with_p1_probability(0.01))
    .run(1000))
```

### Free Functions Provided

| Function | Returns | Purpose |
|----------|---------|---------|
| `sim_builder()` | `SimBuilder` | Create simulation builder |
| `qasm_engine()` | `QasmEngineBuilder` | Create QASM engine builder |
| `llvm_engine()` | `LlvmEngineBuilder` | Create LLVM engine builder |
| `selene_engine()` | `SeleneEngineBuilder` | Create Selene engine builder |
| `sparse_stab()` | `SparseStabBuilder` | Create sparse stabilizer quantum engine |
| `state_vector()` | `StateVectorBuilder` | Create state vector quantum engine |
| `depolarizing_noise()` | `DepolarizingNoiseBuilder` | Create depolarizing noise model |
| `biased_depolarizing_noise()` | `BiasedDepolarizingNoiseBuilder` | Create biased depolarizing noise |
| `general_noise()` | `GeneralNoiseBuilder` | Create general noise model |

### Type Naming Conventions

#### Rust Types (with Builder suffix)
- **Engine Builders**: `QasmEngineBuilder`, `LlvmEngineBuilder`, `SeleneEngineBuilder`
- **Quantum Engine Builders**: `SparseStabBuilder`, `StateVectorBuilder`
- **Noise Builders**: `DepolarizingNoiseBuilder`, `BiasedDepolarizingNoiseBuilder`
- **Built Types**: `QasmEngine`, `SparseStabQuantumEngine`, `DepolarizingNoise`

#### Python Types (without Builder suffix)
- **Engine Builders**: `QasmEngine`, `LlvmEngine`, `SeleneEngine`
- **Quantum Engine Builders**: `SparseStab`, `StateVector`
- **Noise Builders**: `DepolarizingNoise`, `BiasedDepolarizingNoise`

### Design Principles

1. **Consistent API**: Free functions provide identical APIs across languages
2. **Idiomatic Types**: Each language follows its own conventions for type names
3. **Progressive Disclosure**: Simple free functions for common use, types available for advanced use
4. **Builder Pattern**: All configuration uses the builder pattern internally
5. **Separation of Concerns**: Builders (configuration) are separate from built objects (execution)

## Architecture

### Meta-Crate Pattern

PECOS uses a meta-crate pattern where implementation lives in focused sub-crates, and the main `pecos` crate provides a unified, well-organized API:

```
pecos/                    # Meta-crate (facade)
├── pecos-core/          # Core types, traits, errors
├── pecos-engines/       # Engine traits, quantum backends, noise
├── pecos-qasm/          # QASM parser and engine
├── pecos-llvm-sim/      # LLVM simulation engine
├── pecos-selene/        # Selene integration
├── pecos-programs/      # Shared program types
└── pecos-decoders/      # Decoder implementations (optional)
```

Benefits:
- **Separate Compilation**: Each crate compiles independently
- **Optional Dependencies**: Users can depend on individual crates
- **Clear Boundaries**: Each crate has focused responsibility
- **Clean Public API**: Users see organized namespaces, not implementation details

### Layer 0: Shared Program Types (in `pecos-programs` crate)

A zero-dependency crate providing pure data types for programs:

```rust
// In pecos-programs crate (zero dependencies)
pub struct QasmProgram {
    pub source: String,
}

pub struct LlvmProgram {
    pub ir: String,
}

pub struct HugrProgram {
    pub hugr: Vec<u8>, // or appropriate zero-dep representation
}

impl QasmProgram {
    pub fn from_string(s: impl Into<String>) -> Self { ... }
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, std::io::Error> { ... }
}

impl LlvmProgram {
    pub fn from_string(s: impl Into<String>) -> Self { ... }
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, std::io::Error> { ... }
}

impl HugrProgram {
    pub fn from_bytes(bytes: Vec<u8>) -> Self { ... }
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, std::io::Error> { ... }
}
```

### Layer 1: Engine-Specific APIs (per crate)

Each engine crate provides:
- `XEngineProgram`: Data type holding program and engine-specific configuration
- `XEngineBuilder`: Builder for creating and configuring the engine
- `x_engine()`: Factory function returning the builder

#### Example: QASM Engine (in `pecos-qasm` crate)

```rust
use pecos_programs::QasmProgram;

pub struct QasmEngineProgram {
    source: QasmSource,
    // QASM-specific configuration
    virtual_includes: Vec<(String, String)>,
    allow_complex_conditionals: bool,
}

impl QasmEngineProgram {
    // Input methods
    pub fn from_string(s: impl Into<String>) -> Self { ... }
    pub fn from_file(path: impl AsRef<Path>) -> Self { ... }
    pub fn from_bytes(bytes: Vec<u8>) -> Self { ... }

    // Configuration methods
    pub fn with_virtual_includes(mut self, includes: Vec<(String, String)>) -> Self { ... }
    pub fn allow_complex_conditionals(mut self, allow: bool) -> Self { ... }
}

// Automatic conversion from shared program type
impl From<QasmProgram> for QasmEngineProgram {
    fn from(program: QasmProgram) -> Self {
        Self {
            source: QasmSource::String(program.source),
            virtual_includes: vec![],
            allow_complex_conditionals: false,
        }
    }
}

pub struct QasmEngineBuilder {
    program: Option<QasmEngineProgram>,
}

impl QasmEngineBuilder {
    pub fn new() -> Self {
        Self { program: None }
    }

    // Can accept both QasmProgram and QasmEngineProgram
    pub fn program<P: Into<QasmEngineProgram>>(mut self, program: P) -> Self {
        self.program = Some(program.into());
        self
    }

    // Deprecated: Use sim_builder() instead
    pub fn to_sim(self) -> SimBuilder<QasmEngine> { ... }
}

// Free function for ergonomic API
pub fn qasm_engine() -> QasmEngineBuilder {
    QasmEngineBuilder::new()
}
```

#### Example: Selene Engine (in `pecos-selene` crate)

```rust
use pecos_programs::{HugrProgram, LlvmProgram};

pub struct SeleneEngineProgram {
    source: SeleneSource,
    // Selene-specific configuration
    optimization_level: OptLevel,
    debug_info: bool,
}

enum SeleneSource {
    Hugr(Hugr),
    HugrBytes(Vec<u8>),
    LlvmIr(String),
    LlvmFile(PathBuf),
}

impl SeleneEngineProgram {
    // Multiple input types supported
    pub fn from_hugr(hugr: Hugr) -> Self { ... }
    pub fn from_hugr_bytes(bytes: Vec<u8>) -> Self { ... }
    pub fn from_llvm_ir(ir: impl Into<String>) -> Self { ... }
    pub fn from_llvm_file(path: impl AsRef<Path>) -> Self { ... }

    // Configuration
    pub fn with_optimization(mut self, level: OptLevel) -> Self { ... }
    pub fn with_debug_info(mut self, enabled: bool) -> Self { ... }
}

// Automatic conversions from shared program types
impl From<HugrProgram> for SeleneEngineProgram {
    fn from(program: HugrProgram) -> Self {
        Self {
            source: SeleneSource::HugrBytes(program.hugr),
            optimization_level: OptLevel::default(),
            debug_info: false,
        }
    }
}

impl From<LlvmProgram> for SeleneEngineProgram {
    fn from(program: LlvmProgram) -> Self {
        Self {
            source: SeleneSource::LlvmIr(program.ir),
            optimization_level: OptLevel::default(),
            debug_info: false,
        }
    }
}

pub struct SeleneEngineBuilder {
    program: Option<SeleneEngineProgram>,
}

impl SeleneEngineBuilder {
    pub fn new() -> Self {
        Self { program: None }
    }

    // Can accept HugrProgram, LlvmProgram, or SeleneEngineProgram
    pub fn program<P: Into<SeleneEngineProgram>>(mut self, program: P) -> Self {
        self.program = Some(program.into());
        self
    }

    // Deprecated: Use sim_builder() instead
    pub fn to_sim(self) -> SimBuilder<SeleneEngine> { ... }
}

// Free function for ergonomic API
pub fn selene_engine() -> SeleneEngineBuilder {
    SeleneEngineBuilder::new()
}
```

### Layer 2: Unified API (in `pecos` crate)

The unified API re-exports the shared program types and provides automatic engine selection.

#### Program Types

```rust
// Re-export shared program types
pub use pecos_programs::{QasmProgram, LlvmProgram, HugrProgram};

// Enum for runtime dispatch
pub enum Program {
    Qasm(QasmProgram),
    Llvm(LlvmProgram),
    Hugr(HugrProgram),
}
```

#### Unified Simulation Builder

```rust
pub fn sim() -> UnifiedSimBuilder { ... }

impl UnifiedSimBuilder {
    pub fn program<P: Into<Program>>(mut self, program: P) -> Self { ... }
    pub fn cengine<E: ClassicalControlEngineBuilder>(mut self, engine: E) -> Self { ... }
    pub fn seed(mut self, seed: u64) -> Self { ... }
    pub fn workers(mut self, workers: usize) -> Self { ... }
    pub fn run(self, shots: usize) -> Result<ShotVec, PecosError> { ... }
}
```

## Usage Patterns

### Python Usage Examples

```python
# Import namespaces
from pecos_rslib import classical, quantum, noise, programs

# Simple case (using namespaces - recommended)
results = (sim_builder()
    .classical(classical.qasm()
        .program(programs.QasmProgram.from_string(qasm)))
    .run(1000))

# With configuration (using namespaces - recommended)
results = (sim_builder()
    .classical(classical.qasm()
        .program(programs.QasmProgram.from_string(qasm)))
    .quantum(quantum.sparse_stab())
    .noise(noise.depolarizing()
        .with_prep_probability(0.001)
        .with_meas_probability(0.001)
        .with_p1_probability(0.01)
        .with_p2_probability(0.01))
    .seed(42)
    .workers(4)
    .run(1000))

# Builder pattern supports both styles
sim = sim_builder().classical(classical.qasm().program(prog))
sim.seed(42)        # In-place mutation
sim.workers(4)      # Returns self for optional chaining
results = sim.run(1000)

# Alternative: Direct imports (backward compatible)
from pecos_rslib import sim_builder, qasm_engine, sparse_stab, depolarizing_noise
results = (sim_builder()
    .classical(qasm_engine()
        .program(QasmProgram.from_string(qasm)))
    .quantum(sparse_stab())
    .noise(depolarizing_noise().with_p1_probability(0.01))
    .run(1000))
```

### Rust Usage Examples

#### With Proposed Namespace Organization

```rust
use pecos::{classical, quantum, noise, programs};

// Clear, organized API
let results = sim_builder()
    .classical(classical::qasm()
        .program(programs::QasmProgram::from_string(qasm)))
    .quantum(quantum::sparse_stabilizer())
    .noise(noise::depolarizing()
        .with_prep_probability(0.001)
        .with_p1_probability(0.01))
    .run(1000)?;

// Or import specific items
use pecos::{
    classical::qasm_engine,
    quantum::sparse_stab,
    noise::DepolarizingNoiseModelBuilder,
};
```

### 1. Simple Cases (Unified API)

```rust
// Auto-detects QASM, uses default QASM engine
sim()
    .program(QasmProgram::from_file("circuit.qasm"))
    .run(1000)?;

// HUGR auto-selects Selene engine
sim()
    .program(HugrProgram::from_bytes(hugr_bytes))
    .run(1000)?;
```

### 2. Engine Override (Unified API)

```rust
// Use Selene engine for LLVM IR instead of default LLVM engine
sim()
    .program(LlvmProgram::from_string(my_ir))
    .cengine(selene_engine())
    .run(1000)?;

// Pass configured engine
sim()
    .program(LlvmProgram::from_string(my_ir))
    .cengine(selene_engine().with_optimization(OptLevel::Aggressive))
    .run(1000)?;
```

### 3. Full Control (Engine-Specific API)

```rust
// Direct engine API with all options (using free functions)
sim_builder()
    .classical(selene_engine()
        .program(
            SeleneEngineProgram::from_llvm_ir(my_ir)
                .with_optimization(OptLevel::Aggressive)
                .with_debug_info(true)
        ))
    .seed(42)
    .workers(8)
    .quantum(state_vector())
    .noise(depolarizing_noise().with_p1_probability(0.01))
    .run(1000)?;

// QASM with virtual includes
sim_builder()
    .classical(qasm_engine()
        .program(
            QasmEngineProgram::from_file("circuit.qasm")
                .with_virtual_includes(vec![
                    ("custom.inc", custom_gates_definition)
                ])
        ))
    .run(1000)?;

// But engines also accept shared program types!
sim_builder()
    .classical(qasm_engine()
        .program(QasmProgram::from_file("circuit.qasm")))  // Automatic conversion
    .run(1000)?;

// Alternative: Using direct instantiation
SimBuilder::new()
    .classical(SeleneEngineBuilder::new()
        .program(HugrProgram::from_bytes(hugr_bytes)))
    .quantum(StateVectorBuilder::new())
    .run(1000)?;
```

### 4. Multiple Programs

```rust
// Engine-specific (using free functions)
sim_builder()
    .classical(qasm_engine()
        .programs(vec![
            QasmEngineProgram::from_file("definitions.qasm"),
            QasmEngineProgram::from_file("circuit.qasm"),
        ]))
    .run(1000)?;

// Or incremental
let mut engine = qasm_engine();
for file in library_files {
    engine = engine.add_program(QasmEngineProgram::from_file(file));
}
sim_builder().classical(engine).run(1000)?;
```

## Translation Layer

The shared program types from `pecos-programs` are automatically converted to engine-specific types:

```rust
// Conversion traits implemented by engine crates
impl From<pecos_programs::QasmProgram> for pecos_qasm::QasmEngineProgram { ... }
impl From<pecos_programs::LlvmProgram> for pecos_llvm_sim::LlvmEngineProgram { ... }
impl From<pecos_programs::LlvmProgram> for pecos_selene::SeleneEngineProgram { ... }
impl From<pecos_programs::HugrProgram> for pecos_selene::SeleneEngineProgram { ... }
```

## Default Engine Selection

| Program Type | Default Engine | Alternative Engines |
|--------------|----------------|-------------------|
| QasmProgram  | QASM Engine    | -                 |
| LlvmProgram  | LLVM Engine    | Selene Engine     |
| HugrProgram  | Selene Engine  | -                 |

## Implementation Changes for Free Function API

### What Stays the Same

1. **Rust Types**: All existing Builder types remain unchanged
2. **Free Functions**: Already exist in Rust (`qasm_engine()`, `sparse_stab()`, etc.)
3. **Core Functionality**: No changes to actual engine implementations

### What Changes

#### Python Side
1. **Add Free Functions**: Mirror Rust's free functions
   ```python
   def qasm_engine() -> QasmEngineBuilder:
       return QasmEngineBuilder()

   def sparse_stab() -> SparseStabBuilder:
       return SparseStabBuilder()
   ```

2. **Keep Class API**: Python classes remain available without "Builder" suffix
   ```python
   class QasmEngine:  # Wraps QasmEngineBuilder
   class SparseStab:  # Wraps SparseStabBuilder
   ```

#### Documentation
1. **Update Examples**: Show free functions as primary API
2. **Update Tutorials**: Demonstrate consistent cross-language usage

## Namespace Implementation

### Python Implementation (Completed)

Python now has namespace modules that organize functionality:

```python
pecos_rslib/
├── classical.py    # Classical control engine builders
├── noise.py        # Noise model builders
├── quantum.py      # Quantum engine builders
└── programs.py     # Program types
```

Features:
- **Method Chaining**: Builders return `self` for chaining
- **In-place Mutation**: Methods also mutate the builder
- **Backward Compatible**: Direct imports still work

### Rust Implementation (Proposed)

The `pecos` meta-crate would provide namespace modules:

```rust
// In pecos/src/lib.rs
pub mod classical {
    pub use pecos_qasm::{qasm_engine, QasmEngineBuilder};
    pub use pecos_llvm_sim::{llvm_engine, LlvmEngineBuilder};
    pub use pecos_selene::{selene_engine, SeleneEngineBuilder};

    // Convenience aliases
    pub use qasm_engine as qasm;
    pub use llvm_engine as llvm;
    pub use selene_engine as selene;
}

pub mod quantum {
    pub use pecos_engines::quantum_engine_builder::{
        state_vector, sparse_stabilizer, sparse_stab,
        StateVectorEngineBuilder, SparseStabilizerEngineBuilder,
    };
}

pub mod noise {
    pub use pecos_engines::noise::{
        GeneralNoiseModelBuilder, DepolarizingNoiseModelBuilder,
        BiasedDepolarizingNoiseModelBuilder,
    };

    // Convenience functions
    pub fn general() -> GeneralNoiseModelBuilder { ... }
    pub fn depolarizing() -> DepolarizingNoiseModelBuilder { ... }
}

pub mod programs {
    pub use pecos_programs::{QasmProgram, LlvmProgram, HugrProgram};
}

// Backward compatibility: re-export at root
pub use classical::*;
pub use quantum::*;
pub use noise::*;
pub use programs::*;
```

Benefits:
- **Consistent API**: Same namespace structure as Python
- **Better Docs**: Each namespace has its own documentation page
- **Cleaner Imports**: `use pecos::{engines, quantum, noise};`
- **Backward Compatible**: Flat exports still available

## Implementation Plan

### Phase 0: Create pecos-programs Crate
1. Create new `pecos-programs` crate with zero dependencies
2. Implement `QasmProgram`, `LlvmProgram`, `HugrProgram` data types
3. Add to workspace and update dependencies

### Phase 1: Update Engine Crates
1. Add dependency on `pecos-programs`
2. Add `XEngineProgram` types to each engine crate
3. Implement `From<SharedProgram>` conversions
4. Update `.program()` method to accept `Into<XEngineProgram>`
5. Maintain backward compatibility temporarily (can remove later)

### Phase 2: Update Unified API
1. Re-export program types from `pecos-programs` in `pecos` crate
2. Update `UnifiedSimBuilder` to use shared types
3. Update automatic engine selection logic

### Phase 3: Integration
1. Wire up automatic engine selection
2. Add tests for all usage patterns
3. Update documentation

### Phase 4: Implement Rust Namespace Organization
1. Add namespace modules to `pecos` crate
2. Re-export from sub-crates into namespaces
3. Add convenience functions where needed
4. Update documentation with namespace examples

### Phase 5: Fix Lazy Evaluation Implementation
1. Remove `QuantumEngineType` enum from `SimConfig`
2. Add `quantum_engine: Box<dyn QuantumEngine>` to `Simulation` struct
3. Use `quantum_engine_factory` in `build()` method
4. Remove `IntoQuantumEngine` implementation for enums
5. Ensure all methods follow the basic data / builder pattern

### Phase 6: Cleanup
1. Remove deprecated APIs
2. Update examples to use namespaces
3. Migration guide for namespace usage

## Future Extensions

1. **Streaming Programs**: Support for programs provided in chunks
2. **Remote Programs**: URLs, database references
3. **Program Validation**: Pre-flight checks before engine creation
4. **Program Optimization**: Cross-engine optimizations
5. **Caching**: Compiled program caching

## Benefits

1. **Ease of Use**: `sim().program(...).run()` for simple cases
2. **Flexibility**: Override engine selection when needed
3. **Type Safety**: Engine-specific APIs preserve type information
4. **Performance**: Zero-copy for in-memory objects
5. **Extensibility**: Easy to add new formats and engines
6. **Decoupling**: Formats and engines evolve independently
7. **Code Reuse**: Shared program types eliminate duplication
8. **Automatic Conversions**: Engines accept both shared and engine-specific types seamlessly
9. **Discoverability**: Namespace organization makes APIs easier to explore
10. **Consistency**: Same namespace structure across Python and Rust
11. **Method Chaining**: Builders support both mutation and chaining patterns
12. **Lazy Construction**: Engines and models are only built when needed
13. **Clear Input Types**: Methods accept either basic data or builders, never mixed types
