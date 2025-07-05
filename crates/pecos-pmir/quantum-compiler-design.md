# Quantum-Classical Hybrid Compiler Design

original discussion: https://claude.ai/chat/0a1a9e6a-e719-4c8b-9906-d955e8d480fe

## Overview

This document describes a high-performance compiler architecture for hybrid quantum-classical programs. The design emphasizes:
- Avoiding C++ by using Rust throughout
- Leveraging MLIR concepts without direct C++ integration
- Data-oriented design (DOD) and Entity Component System (ECS) for fast simulation
- Flexible compilation strategies for different targets

## Architecture Overview

```
Frontend Languages/LLVM IR → 
Universal MLIR-like AST/IR (Rust) →
Analysis & Optimization →
Code Generation →
├── Simulation: Specialized Rust code with inlined operations
└── Hardware: MLIR text → quantum hardware backend
└── Other Frontend Langues/LLVM IR
```

## Core IR Design

### Universal MLIR-like IR in Rust

```rust
// Core hierarchical structure inspired by MLIR
#[derive(Clone, Debug)]
pub struct Module {
    pub functions: Vec<Function>,
    pub quantum_registers: Vec<QuantumRegister>,
    pub metadata: Metadata,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub signature: FunctionType,
    pub body: Region,
    pub attributes: Attributes,
}

#[derive(Clone, Debug)]
pub struct Region {
    pub blocks: Vec<Block>,
    pub region_type: RegionType,
}

#[derive(Clone, Debug)]
pub enum RegionType {
    Classical,      // Pure classical computation
    Quantum,        // Pure quantum operations
    Hybrid,         // Mixed classical-quantum
    ControlFlow(Box<ControlFlowType>),
}

#[derive(Clone, Debug)]
pub struct Block {
    pub label: Option<String>,
    pub arguments: Vec<BlockArgument>,
    pub operations: Vec<Operation>,
    pub terminator: Terminator,
}
```

### Flexible Operation System

```rust
#[derive(Clone, Debug)]
pub enum Operation {
    // Classical operations - maps directly to LLVM
    Classical(ClassicalOp),
    
    // Quantum operations - common across quantum languages
    Quantum(QuantumOp),
    
    // Measurement bridges quantum → classical
    Measurement {
        qubits: Vec<QubitRef>,
        targets: Vec<ClassicalRef>,
        basis: MeasurementBasis,
    },
    
    // Noise operations for simulation
    Noise(NoiseOp),
    
    // Control flow that may involve quantum conditions
    ControlFlow(ControlFlowOp),
    
    // Extension point for language-specific operations
    Custom(Box<dyn CustomOperation>),
}

#[derive(Clone, Debug)]
pub enum QuantumOp {
    SingleQubitGate {
        gate: Gate,
        target: QubitRef,
        params: Vec<f64>,
    },
    MultiQubitGate {
        gate: Gate,
        targets: Vec<QubitRef>,
        params: Vec<f64>,
    },
    QuantumCircuit {
        subcircuit: Region,
    },
    Extended(Box<dyn QuantumExtension>),
}

#[derive(Clone, Debug)]
pub enum NoiseOp {
    Depolarizing { target: QubitRef, probability: f64 },
    AmplitudeDamping { target: QubitRef, gamma: f64 },
    PhaseDamping { target: QubitRef, gamma: f64 },
    TwoQubitDepolarizing { targets: [QubitRef; 2], probability: f64 },
    KrausChannel { targets: Vec<QubitRef>, operators: Vec<KrausOperator> },
    CrosstalkNoise { primary: QubitRef, affected: Vec<(QubitRef, f64)> },
}
```

### Type System

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    // Classical types - map to LLVM
    Integer { bits: u32, signed: bool },
    Float { bits: u32 },
    Pointer(Box<Type>),
    Array { element: Box<Type>, size: Option<usize> },
    Struct { fields: Vec<Type> },
    
    // Quantum types
    Qubit,
    QuantumRegister { size: usize },
    QuantumState { dim: usize },
    
    // Hybrid types
    MeasurementResult,
    ClassicalRegister { bits: u32 },
    
    // Extension point
    Custom(String, Box<dyn Any>),
}
```

## Compilation Pipeline

### 1. Frontend Integration

```rust
pub trait QuantumLanguageFrontend {
    fn parse(&self, source: &str) -> Result<ProgramAST, Error>;
    fn lower_to_ir(&self, ast: ProgramAST) -> Result<Module, Error>;
    fn optimize_ir(&self, module: &mut Module) -> Result<(), Error> {
        Ok(()) // Default: no language-specific optimizations
    }
}
```

### 2. Analysis Phase

```rust
pub struct ProgramAnalyzer {
    fn analyze(&self, module: &Module) -> ProgramProfile {
        ProgramProfile {
            quantum_density: self.compute_quantum_density(module),
            classical_complexity: self.analyze_classical_regions(module),
            access_patterns: self.analyze_memory_access(module),
            parallelism_opportunities: self.find_parallel_regions(module),
            noise_characteristics: self.analyze_noise_ops(module),
        }
    }
}

pub enum ProgramProfile {
    QuantumCircuit,           // Circuit-like: many gates, little classical
    VariationalAlgorithm,     // VQE/QAOA: repeated parameterized circuits
    QuantumClassicalHybrid,   // Complex interleaving of quantum/classical
}
```

### 3. Optimization Strategy

```rust
impl Module {
    pub fn optimize(&mut self) -> Result<(), Error> {
        // High-level optimizations
        self.fuse_adjacent_gates()?;
        self.eliminate_redundant_operations()?;
        self.fold_constants()?;
        
        // Region-based optimization
        for function in &mut self.functions {
            function.body.split_regions();
            
            for region in function.body.all_regions_mut() {
                match region.region_type {
                    RegionType::Classical => {
                        region.prepare_for_compilation();
                    }
                    RegionType::Quantum => {
                        region.optimize_quantum_gates();
                        region.batch_similar_operations();
                    }
                    RegionType::Hybrid => {
                        region.try_split_further();
                    }
                    _ => {}
                }
            }
        }
        
        Ok(())
    }
}
```

## Fast Simulation Strategy

### Core Principle: Generate Specialized Rust Code

Instead of using LLVM JIT with FFI overhead, generate specialized Rust code that directly manipulates quantum state:

```rust
pub struct OptimizedCompiler {
    fn compile_for_simulation(&self, module: &Module) -> Result<String, Error> {
        let mut codegen = CodeGenerator::new();
        
        // Generate imports and setup
        codegen.add_imports(&[
            "use quantum_sim::*;",
            "use packed_simd::*;",
            "#[cfg(feature = \"noise\")]",
            "use quantum_sim::noise::*;",
        ]);
        
        // Generate main simulation function
        codegen.function("simulate", |gen| {
            // Pre-allocate all state vectors
            gen.line("let mut state = StateVector::aligned_new(NUM_QUBITS);");
            gen.line("let mut classical_regs = ClassicalRegisters::new();");
            
            // Inline all operations
            for region in module.all_regions() {
                match region.region_type {
                    RegionType::Classical => {
                        gen.inline_classical_operations(&region.operations);
                    }
                    RegionType::Quantum => {
                        gen.inline_quantum_operations(&region.operations);
                    }
                }
            }
        });
        
        Ok(codegen.build())
    }
}
```

### Data-Oriented State Representation

```rust
// Cache-friendly state vector layout
#[repr(align(64))]  // Cache line aligned
pub struct StateVector {
    pub data: Vec<Complex64>,  // Or separate real/imag for better SIMD
    pub num_qubits: u32,
}

// Alternative layout for better vectorization
pub struct StateVectorSOA {
    pub real_parts: AlignedVec<f64>,
    pub imag_parts: AlignedVec<f64>,
    pub num_qubits: u32,
}

// Gate operations as data
#[repr(C)]
pub struct GateOperation {
    pub gate_type: GateType,
    pub target_qubits: [u32; 4],  // Fixed size for cache
    pub parameters: [f64; 4],
    pub active_qubits: u8,
}
```

### ECS Architecture for Simulation

```rust
// Components
#[derive(Component)]
struct QuantumAmplitudes(AlignedVec<Complex64>);

#[derive(Component)]
struct ClassicalState(Vec<u64>);

#[derive(Component)]
struct NoiseModel {
    t1: f64,
    t2: f64,
    gate_errors: HashMap<GateType, f64>,
}

#[derive(Component)]
struct GateQueue(Vec<GateOperation>);

// Systems
pub struct GateFusionSystem;
impl System for GateFusionSystem {
    fn run(&mut self, world: &mut World) {
        let mut query = world.query::<&mut GateQueue>();
        for mut queue in query.iter_mut(world) {
            queue.0 = fuse_gates(&queue.0);
        }
    }
}

pub struct QuantumEvolutionSystem;
impl System for QuantumEvolutionSystem {
    fn run(&mut self, world: &mut World) {
        let mut query = world.query::<(&mut QuantumAmplitudes, &GateQueue)>();
        for (mut amplitudes, gates) in query.iter_mut(world) {
            for gate in &gates.0 {
                apply_gate_vectorized(&mut amplitudes.0, gate);
            }
        }
    }
}
```

### Inlined Gate Application

```rust
impl CodeGenerator {
    fn inline_quantum_gate(&mut self, gate: &QuantumOp) {
        match gate {
            QuantumOp::SingleQubitGate { gate: Gate::H, target, .. } => {
                // Direct inlined Hadamard
                self.lines(&[
                    &format!("// Hadamard on qubit {}", target),
                    &format!("let stride = 1usize << {};", target),
                    &format!("let pairs = state.len() >> {};", target + 1),
                    "for pair_idx in 0..pairs {",
                    "    let i = pair_idx * 2 * stride + (pair_idx / stride) * stride;",
                    "    let j = i + stride;",
                    "    let (a, b) = (state.data[i], state.data[j]);",
                    "    state.data[i] = SQRT_HALF * (a + b);",
                    "    state.data[j] = SQRT_HALF * (a - b);",
                    "}",
                ]);
            }
            // Other gates...
        }
    }
    
    fn inline_with_vectorization(&mut self, gate: &QuantumOp) {
        // Generate SIMD-friendly code
        self.lines(&[
            "#[cfg(target_arch = \"x86_64\")]",
            "{",
            "    use std::arch::x86_64::*;",
            "    unsafe {",
            "        // Process 4 complex amplitudes at once",
            "        let chunks = state.data.chunks_exact_mut(4);",
            "        for chunk in chunks {",
            "            let vals = _mm256_loadu_pd(chunk.as_ptr());",
            "            let result = apply_gate_simd(vals, gate_matrix);",
            "            _mm256_storeu_pd(chunk.as_mut_ptr(), result);",
            "        }",
            "    }",
            "}",
        ]);
    }
}
```

### Noise Handling

```rust
impl CodeGenerator {
    fn inline_noise_operations(&mut self, noise: &NoiseOp, conditional: bool) {
        if conditional {
            self.line("#[cfg(feature = \"noise-simulation\")]");
            self.line("{");
            self.indent();
        }
        
        match noise {
            NoiseOp::Depolarizing { target, probability } => {
                self.lines(&[
                    "let r = rng.gen::<f64>();",
                    &format!("if r < {} {{", probability / 3.0),
                    &format!("    apply_pauli_x_inlined(&mut state, {});", target),
                    &format!("}} else if r < {} {{", 2.0 * probability / 3.0),
                    &format!("    apply_pauli_y_inlined(&mut state, {});", target),
                    &format!("}} else if r < {} {{", probability),
                    &format!("    apply_pauli_z_inlined(&mut state, {});", target),
                    "}",
                ]);
            }
            // Other noise models...
        }
        
        if conditional {
            self.dedent();
            self.line("}");
        }
    }
}
```

## Compilation and Execution

### Compile Generated Rust Code

```rust
pub fn compile_optimized_simulation(rust_code: &str) -> Result<Library, Error> {
    let temp_dir = tempdir()?;
    let src_path = temp_dir.path().join("simulation.rs");
    fs::write(&src_path, rust_code)?;
    
    // Compile with maximum optimizations
    Command::new("rustc")
        .args(&[
            "--crate-type=cdylib",
            "-C", "opt-level=3",
            "-C", "target-cpu=native",
            "-C", "lto=fat",
            "-C", "codegen-units=1",
            "-C", "target-feature=+avx2,+fma",
        ])
        .arg(&src_path)
        .status()?;
    
    // Load and execute
    unsafe { Library::new(output_path) }
}
```

### Hardware Target Compilation

For quantum hardware, strip noise and generate appropriate format:

```rust
impl Module {
    fn compile_for_hardware(&self, target: HardwareTarget) -> Result<String, Error> {
        // Strip noise operations
        let cleaned = self.strip_noise_ops();
        
        // Generate MLIR text for hardware backend
        let mlir = cleaned.to_mlir();
        
        // Use mlir-opt for hardware-specific lowering
        let hardware_ir = run_mlir_tool(&[
            "mlir-opt",
            "--lower-to-hardware-dialect",
            "--hardware-target", &target.name(),
        ], &mlir)?;
        
        Ok(hardware_ir)
    }
}
```

## Performance Optimizations Summary

1. **Avoid FFI Overhead**: Generate pure Rust code instead of LLVM JIT + FFI
2. **Inline Everything**: No function calls in hot paths
3. **Data-Oriented Layout**: Cache-friendly state vector representation
4. **ECS Architecture**: Flexible composition and parallel systems
5. **Compile-Time Optimization**: Let rustc/LLVM optimize the entire program
6. **Conditional Noise**: Strip noise at compile time for hardware targets
7. **SIMD Utilization**: Generate vectorization-friendly code patterns

## Expected Performance

- **vs Interpretation**: 10-50x faster
- **vs LLVM JIT with FFI**: 2-5x faster for quantum-heavy code
- **vs Generic Simulator**: 3-10x faster due to specialization
- **Hardware Deployment**: Zero overhead (noise stripped at compile time)

## Development Strategy: Debug vs Production Paths

### Debug Path (Development/Testing)

For rapid development and debugging, support direct LLVM JIT execution with linked Rust quantum functions:

```rust
pub struct DebugExecutor {
    llvm_context: LLVMContext,
    quantum_sim: QuantumSimulator,
}

impl DebugExecutor {
    pub fn setup_quantum_functions(&mut self) -> Result<(), Error> {
        // Register Rust simulator functions as LLVM externals
        self.llvm_context.add_external_function(
            "quantum_h", quantum_h_gate as *const u8
        );
        // ... other quantum operations
    }
    
    pub fn execute_llvm_ir(&mut self, llvm_ir: &str) -> Result<(), Error> {
        let module = self.llvm_context.parse_ir(llvm_ir)?;
        let jit = self.llvm_context.create_jit(module)?;
        
        unsafe {
            let main_fn = jit.get_function::<fn(*mut c_void)>("main")?;
            main_fn(&mut self.quantum_sim as *mut _ as *mut c_void);
        }
        Ok(())
    }
}

// External functions callable from LLVM
extern "C" fn quantum_h_gate(sim: *mut c_void, qubit: u32) {
    let sim = unsafe { &mut *(sim as *mut QuantumSimulator) };
    sim.apply_h(qubit as usize);
}
```

Benefits:
- Quick testing without full compilation pipeline
- Easy debugging with familiar LLVM tools
- Validates quantum algorithms before optimization

### Production Path (Performance)

The full pipeline for maximum performance:

```rust
pub struct ProductionCompiler {
    pub fn compile(&self, source: Source, target: Target) -> Result<Executable, Error> {
        // 1. Parse to MLIR-like IR
        let ir = self.parse_to_ir(source)?;
        
        // 2. Optimize at IR level
        let optimized = self.optimize_ir(ir)?;
        
        // 3. Generate specialized code
        match target {
            Target::RustSimulation => {
                let rust_code = self.generate_inlined_rust(&optimized)?;
                compile_rust_code(rust_code)
            }
            Target::LLVMIR => optimized.to_llvm_ir(),
            Target::QuantumHardware => self.generate_hardware_code(&optimized),
        }
    }
}
```

### Unified Development Workflow

```rust
pub struct HybridCompiler {
    debug_mode: bool,
    
    pub fn execute(&mut self, source: Source, options: CompileOptions) -> Result<(), Error> {
        if self.debug_mode {
            // Quick path for development
            self.execute_debug_path(source)
        } else {
            // Optimized production path
            self.execute_production_path(source, options.target)
        }
    }
    
    // Validation: run both paths and compare
    pub fn validate(&self, source: Source) -> Result<(), Error> {
        let debug_result = self.run_debug_path(source.clone())?;
        let prod_result = self.run_production_path(source)?;
        assert_eq!(debug_result.quantum_state, prod_result.quantum_state);
        Ok(())
    }
}
```

This dual approach enables:
1. **Fast iteration** during development
2. **Correctness validation** by comparing paths
3. **Performance analysis** to verify optimization benefits
4. **Incremental migration** from debug to production

## Implementation Workflow

### Phase 1: Debug Infrastructure
1. Set up LLVM JIT with linked Rust quantum functions
2. Basic quantum operation support
3. Simple test framework

### Phase 2: IR Development
1. Parse frontend languages to individual ASTs
2. Build LLVM-IR → MLIR-like IR translator
3. Implement basic IR optimizations

### Phase 3: Production Pipeline
1. Analyze program structure and characteristics
2. Apply high-level optimizations (gate fusion, constant folding)
3. Generate specialized Rust code with everything inlined
4. Compile with aggressive optimizations

### Phase 4: Multi-target Support
1. Execute directly for simulation
2. Generate MLIR text for hardware backends
3. Support round-trip to LLVM-IR when needed

This architecture provides maximum performance while maintaining flexibility and avoiding C++ complexity.