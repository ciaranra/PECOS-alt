# Quantum Error Correction in PMIR

This document details PMIR's comprehensive support for quantum error correction (QEC) and fault-tolerant quantum computing.

## Table of Contents

1. [Overview](#1-overview)
2. [QEC Type System](#2-qec-type-system)
3. [QEC Operations](#3-qec-operations)
4. [Resource Management](#4-resource-management)
5. [Fault-Tolerant Compilation](#5-fault-tolerant-compilation)
6. [Syndrome Decoding](#6-syndrome-decoding)
7. [Analysis and Optimization](#7-analysis-and-optimization)
8. [Examples](#8-examples)

## 1. Overview

PMIR provides first-class support for quantum error correction, treating QEC not as an afterthought but as a fundamental part of the IR design. This enables:

- Natural expression of logical operations
- Automatic lowering to physical implementations
- Resource estimation for fault-tolerant algorithms
- Integration with classical decoding algorithms
- Support for multiple QEC codes

## 2. QEC Type System

### 2.1 QEC-Specific Types

```rust
pub enum QECType {
    // Logical qubits with error correction
    LogicalQubit {
        code: ErrorCorrectionCode,
        distance: usize,
    },
    
    // Physical qubits (for syndrome extraction)
    PhysicalQubit,
    
    // Syndrome measurement results
    Syndrome {
        code: ErrorCorrectionCode,
        syndrome_type: SyndromeType,
    },
    
    // Ancilla qubits for QEC
    AncillaQubit {
        purpose: AncillaPurpose,
        reset_protocol: ResetProtocol,
    },
    
    // Fault-tolerant regions
    FaultTolerantBlock {
        error_threshold: f64,
        verification_level: usize,
    },
}
```

### 2.2 Error Correction Codes

```rust
pub enum ErrorCorrectionCode {
    Surface { distance: usize },
    Color { distance: usize },
    RepetitionCode { length: usize },
    QuantumLDPC { 
        parity_check_matrix: Matrix,
        logical_operators: LogicalOps,
    },
    Custom { 
        name: String, 
        params: CodeParams 
    },
}

pub struct CodeParams {
    pub physical_qubits: usize,
    pub logical_qubits: usize,
    pub distance: usize,
    pub threshold: f64,
    pub gates: SupportedGates,
}
```

### 2.3 Type Constraints

```rust
// Ensure operations respect QEC boundaries
pub trait QECTypeCheck {
    fn verify_transversal(&self, op: &Operation) -> Result<(), QECError>;
    fn verify_fault_tolerant(&self, sequence: &[Operation]) -> Result<(), QECError>;
    fn check_logical_operation(&self, op: &Operation, code: &ErrorCorrectionCode) -> Result<(), QECError>;
}
```

## 3. QEC Operations

### 3.1 Logical Operations

```rust
pub enum QECOp {
    // Logical qubit initialization
    LogicalInit {
        code: ErrorCorrectionCode,
        state: LogicalState,
        verification_rounds: usize,
    },
    
    // Logical gates
    LogicalGate {
        gate: LogicalGateType,
        qubits: Vec<LogicalQubitRef>,
        transversal: bool,
    },
    
    // Magic state injection
    MagicStateInjection {
        state_type: MagicStateType,
        distillation_level: usize,
        fidelity_target: f64,
    },
    
    // Lattice surgery
    LatticeSurgery {
        operation: SurgeryOp,
        patches: Vec<CodePatch>,
        merge_protocol: MergeProtocol,
    },
}
```

### 3.2 Syndrome Operations

```rust
pub enum SyndromeOp {
    // Extract error syndromes
    SyndromeExtraction {
        logical_qubits: Vec<LogicalQubitRef>,
        syndrome_qubits: Vec<AncillaRef>,
        extraction_circuit: Region,
        rounds: usize,
    },
    
    // Process syndrome data
    SyndromeProcessing {
        raw_syndrome: SyndromeData,
        history_window: usize,
        noise_model: NoiseModel,
    },
    
    // Apply corrections
    ErrorCorrection {
        syndrome: ProcessedSyndrome,
        correction_table: CorrectionTable,
        parallel_decode: bool,
    },
}
```

### 3.3 Fault-Tolerant Operations

```rust
// Example: Fault-tolerant T gate
let ft_t_gate = qec::FaultTolerantGate {
    gate_type: GateType::T,
    implementation: FTImplementation::MagicState {
        magic_factory: factory_ref,
        consumption_rate: 1,
        success_probability: 0.99,
    },
    verification: PostSelectionProtocol {
        rounds: 3,
        threshold: 0.001,
    },
};
```

## 4. Resource Management

### 4.1 Physical Layout

```rust
pub struct QECResourceManager {
    // Physical qubit allocation
    physical_layout: PhysicalQubitLayout,
    
    // Syndrome extraction scheduling
    syndrome_schedule: SyndromeSchedule,
    
    // Classical decoding resources
    decoder_allocation: DecoderResources,
    
    // Magic state factories
    distillation_factories: Vec<DistillationFactory>,
}

pub struct PhysicalQubitLayout {
    // Map logical to physical qubits
    logical_to_physical: HashMap<LogicalQubitId, Vec<PhysicalQubitId>>,
    
    // Connectivity constraints
    connectivity: ConnectivityGraph,
    
    // Reserved ancilla regions
    ancilla_zones: Vec<AncillaZone>,
}
```

### 4.2 Resource Scheduling

```rust
pub struct QECScheduler {
    // Syndrome extraction frequency
    syndrome_interval: usize,
    
    // Preserve fault-tolerance
    ft_constraints: FTConstraints,
    
    // Classical-quantum coordination
    decode_latency_budget: Duration,
    
    pub fn schedule_with_qec(&self, ops: Vec<Operation>) -> Schedule {
        let mut schedule = Schedule::new();
        
        // Group operations by QEC rounds
        let rounds = self.group_into_qec_rounds(ops);
        
        for round in rounds {
            // Schedule quantum operations
            schedule.add_quantum_ops(round.quantum_ops);
            
            // Insert syndrome extraction
            schedule.add_syndrome_extraction();
            
            // Schedule classical decoding (parallel)
            schedule.add_parallel_decoding();
            
            // Apply corrections before next round
            schedule.add_error_correction();
        }
        
        schedule
    }
}
```

### 4.3 Magic State Management

```rust
pub struct MagicStateFactory {
    // Distillation protocol
    protocol: DistillationProtocol,
    
    // Input fidelity
    raw_fidelity: f64,
    
    // Output specifications
    target_fidelity: f64,
    production_rate: f64,
    
    // Resource requirements
    physical_qubits: usize,
    distillation_time: Duration,
}

impl MagicStateFactory {
    pub fn distill_magic_states(&self, count: usize) -> Result<Vec<MagicState>, QECError> {
        // 15-to-1 distillation protocol
        let rounds = (count as f64 / self.production_rate).ceil() as usize;
        
        let mut magic_states = Vec::new();
        for _ in 0..rounds {
            let raw_states = self.prepare_raw_states(15)?;
            let distilled = self.distill_round(raw_states)?;
            magic_states.push(distilled);
        }
        
        Ok(magic_states)
    }
}
```

## 5. Fault-Tolerant Compilation

### 5.1 Compilation Pipeline

```rust
pub struct FaultTolerantCompiler {
    // Code selection strategy
    code_selector: CodeSelector,
    
    // Gate synthesis
    gate_synthesizer: FTGateSynthesizer,
    
    // Resource optimizer
    resource_optimizer: QECResourceOptimizer,
    
    pub fn compile(&self, module: &Module) -> Result<FTModule, Error> {
        // 1. Analyze error requirements
        let error_budget = self.analyze_error_requirements(module)?;
        
        // 2. Choose QEC codes
        let code_assignment = self.code_selector.assign_codes(module, error_budget)?;
        
        // 3. Synthesize fault-tolerant operations
        let ft_ops = self.synthesize_operations(module, &code_assignment)?;
        
        // 4. Insert syndrome extraction
        let with_syndromes = self.insert_syndrome_extraction(ft_ops)?;
        
        // 5. Optimize resource usage
        let optimized = self.resource_optimizer.optimize(with_syndromes)?;
        
        Ok(FTModule {
            operations: optimized,
            code_assignment,
            resource_estimate: self.estimate_resources(&optimized)?,
        })
    }
}
```

### 5.2 Code Selection

```rust
pub struct CodeSelector {
    available_codes: Vec<ErrorCorrectionCode>,
    hardware_constraints: HardwareConstraints,
    
    pub fn select_code(&self, requirements: &QECRequirements) -> ErrorCorrectionCode {
        // Consider multiple factors
        let candidates = self.available_codes.iter()
            .filter(|code| code.meets_threshold(requirements.error_rate))
            .filter(|code| code.supports_gates(&requirements.gate_set))
            .filter(|code| self.hardware_supports(code));
            
        // Optimize for resource usage
        candidates
            .min_by_key(|code| code.resource_cost(requirements))
            .cloned()
            .unwrap_or(ErrorCorrectionCode::Surface { distance: 17 })
    }
}
```

### 5.3 Gate Synthesis

```rust
pub struct FTGateSynthesizer {
    pub fn synthesize_gate(&self, gate: &LogicalGate, code: &ErrorCorrectionCode) -> FTImplementation {
        match (gate, code) {
            // Transversal gates
            (LogicalGate::CNOT, _) => FTImplementation::Transversal,
            (LogicalGate::H, ErrorCorrectionCode::Color { .. }) => FTImplementation::Transversal,
            
            // Gates requiring magic states
            (LogicalGate::T, _) => FTImplementation::MagicState {
                protocol: MagicProtocol::StandardTeleportation,
                magic_type: MagicStateType::T,
            },
            
            // Lattice surgery
            (LogicalGate::CNOT, ErrorCorrectionCode::Surface { .. }) => {
                FTImplementation::LatticeSurgery {
                    merge_type: MergeType::Rough,
                    orientation: Orientation::Horizontal,
                }
            }
            
            // Custom synthesis
            _ => self.custom_synthesis(gate, code),
        }
    }
}
```

## 6. Syndrome Decoding

### 6.1 Decoder Integration

```rust
pub enum DecodingOp {
    // Minimum weight perfect matching
    MWPM {
        syndrome: SyndromeData,
        graph: DecodingGraph,
        parallel_regions: usize,
    },
    
    // Union-Find decoder
    UnionFind {
        syndrome: SyndromeData,
        growth_rate: f64,
        early_termination: bool,
    },
    
    // Machine learning decoder
    NeuralDecoder {
        model: DecoderModel,
        batch_size: usize,
        accelerator: ComputeDevice,
    },
    
    // Belief propagation
    BeliefPropagation {
        syndrome: SyndromeData,
        max_iterations: usize,
        damping_factor: f64,
    },
}
```

### 6.2 Parallel Syndrome Processing

```rust
// MLIR representation of parallel decoding
func @parallel_syndrome_decode(%syndrome: tensor<1000x1000xi1>) -> tensor<1000x1000xi1> {
    // Split syndrome into regions
    %regions = tensor.split %syndrome, %num_decoders : tensor<1000x1000xi1> -> tensor<?x?x?xi1>
    
    // Parallel decoding
    %corrections = parallel.map %regions {
        ^bb0(%region: tensor<?x?xi1>):
            %local_correction = qec.mwpm %region : tensor<?x?xi1> -> tensor<?x?xi1>
            parallel.yield %local_correction
    }
    
    // Merge with boundary resolution
    %merged = qec.merge_corrections %corrections : tensor<?x?x?xi1> -> tensor<1000x1000xi1>
    return %merged : tensor<1000x1000xi1>
}
```

### 6.3 Real-time Decoding

```rust
pub struct RealtimeDecoder {
    // Streaming syndrome processor
    syndrome_buffer: CircularBuffer<SyndromeData>,
    
    // Adaptive decoder
    decoder: Box<dyn AdaptiveDecoder>,
    
    // Performance monitoring
    metrics: DecoderMetrics,
    
    pub fn process_syndrome_stream(&mut self, syndrome: SyndromeData) -> Correction {
        // Add to history
        self.syndrome_buffer.push(syndrome);
        
        // Adaptive decoding with history
        let correction = self.decoder.decode_with_history(
            &syndrome,
            self.syndrome_buffer.window(3)
        );
        
        // Update metrics
        self.metrics.record_latency(correction.decode_time);
        
        correction
    }
}
```

## 7. Analysis and Optimization

### 7.1 Error Rate Analysis

```rust
pub struct ErrorRateAnalyzer {
    noise_model: NoiseModel,
    monte_carlo_samples: usize,
    
    pub fn analyze_logical_error_rate(&self, circuit: &FTCircuit) -> LogicalErrorRate {
        let mut error_counts = HashMap::new();
        
        for _ in 0..self.monte_carlo_samples {
            // Simulate with noise
            let result = self.simulate_with_errors(circuit);
            
            // Track logical errors
            if let Some(error) = result.logical_error {
                *error_counts.entry(error).or_insert(0) += 1;
            }
        }
        
        LogicalErrorRate {
            x_error: error_counts.get(&ErrorType::X).unwrap_or(&0) as f64 / self.monte_carlo_samples as f64,
            z_error: error_counts.get(&ErrorType::Z).unwrap_or(&0) as f64 / self.monte_carlo_samples as f64,
            y_error: error_counts.get(&ErrorType::Y).unwrap_or(&0) as f64 / self.monte_carlo_samples as f64,
        }
    }
}
```

### 7.2 Resource Optimization

```rust
pub struct QECResourceOptimizer {
    optimization_target: OptimizationTarget,
    
    pub fn optimize(&self, circuit: FTCircuit) -> FTCircuit {
        match self.optimization_target {
            OptimizationTarget::MinimizeQubits => {
                self.compact_layout(circuit)
            }
            OptimizationTarget::MinimizeDepth => {
                self.parallelize_operations(circuit)
            }
            OptimizationTarget::MinimizeDecoding => {
                self.batch_syndrome_extraction(circuit)
            }
            OptimizationTarget::Balanced => {
                self.multi_objective_optimize(circuit)
            }
        }
    }
    
    fn compact_layout(&self, circuit: FTCircuit) -> FTCircuit {
        // Minimize physical qubit usage
        let mut optimizer = LayoutOptimizer::new();
        optimizer.pack_logical_qubits(&circuit.logical_qubits);
        optimizer.share_ancilla_qubits(&circuit.syndrome_qubits);
        optimizer.apply_to_circuit(circuit)
    }
}
```

### 7.3 Threshold Analysis

```rust
pub struct ThresholdAnalyzer {
    pub fn find_threshold(&self, code: &ErrorCorrectionCode) -> f64 {
        // Binary search for threshold
        let mut low = 0.0;
        let mut high = 0.1;
        
        while high - low > 1e-6 {
            let mid = (low + high) / 2.0;
            let logical_rate = self.logical_error_rate(code, mid);
            let physical_rate = mid;
            
            if logical_rate < physical_rate {
                low = mid;  // Below threshold
            } else {
                high = mid; // Above threshold
            }
        }
        
        (low + high) / 2.0
    }
}
```

## 8. Examples

### 8.1 Surface Code Implementation

```rust
// Surface code with distance 21
let surface_code = Module {
    name: "surface_code_demo",
    operations: vec![
        // Allocate logical qubit
        qec::allocate_logical(ErrorCorrectionCode::Surface { distance: 21 }),
        
        // Logical Hadamard (requires lattice rotation)
        qec::logical_h_surface(),
        
        // Syndrome extraction round
        qec::extract_syndrome_surface(),
        
        // Decode syndrome
        qec::decode_mwpm(),
        
        // Apply correction
        qec::apply_pauli_correction(),
        
        // Logical measurement
        qec::logical_measure_surface(),
    ],
};
```

### 8.2 Magic State Distillation

```rust
// 15-to-1 magic state distillation
func @distill_magic_state() -> !qec.magic_state {
    // Prepare 15 noisy T states
    %raw_states = qec.prepare_t_states(15) : () -> !qec.magic_state<15>
    
    // Distillation circuit
    %ancilla = qec.prepare_plus_states(4) : () -> !qec.ancilla<4>
    
    // Apply distillation protocol
    %distilled = qec.distill_15_to_1(%raw_states, %ancilla) 
        : (!qec.magic_state<15>, !qec.ancilla<4>) -> !qec.magic_state
    
    // Verify output fidelity
    qec.verify_fidelity(%distilled, 0.9999) : !qec.magic_state
    
    return %distilled : !qec.magic_state
}
```

### 8.3 Fault-Tolerant Algorithm

```rust
// Shor's algorithm with full fault tolerance
let ft_shor = PMIRBuilder::new()
    .with_qec_dialect()
    .with_function("shor_factoring", |f| {
        // Configure QEC
        let qec_config = QECConfig {
            code: ErrorCorrectionCode::Surface { distance: 31 },
            syndrome_interval: 1000,  // Every 1000 gates
            decoder: DecoderType::MWPM,
        };
        
        f.with_qec(qec_config, |qec| {
            // Allocate logical registers
            let n = 2048;  // Number to factor
            let work_qubits = qec.allocate_logical_register(2 * n.bits());
            let ancilla = qec.allocate_logical_register(n.bits());
            
            // Quantum period finding with error correction
            qec.qft_fault_tolerant(&work_qubits);
            qec.modular_exponentiation_ft(n, &work_qubits, &ancilla);
            qec.inverse_qft_fault_tolerant(&work_qubits);
            
            // Extract result with verification
            let result = qec.measure_logical_register(&work_qubits);
            qec.verify_computation(result)
        })
    })
    .build();
```

### 8.4 Resource Estimation

```rust
// Estimate resources for a fault-tolerant circuit
let resource_estimate = ResourceEstimator::new()
    .with_error_model(realistic_noise_model())
    .with_qec_code(ErrorCorrectionCode::Surface { distance: 25 })
    .estimate(&my_algorithm);

println!("Resource Requirements:");
println!("  Physical qubits: {}", resource_estimate.physical_qubits);
println!("  Logical depth: {}", resource_estimate.logical_depth);
println!("  Syndrome rounds: {}", resource_estimate.syndrome_rounds);
println!("  Classical FLOPS: {}", resource_estimate.classical_operations);
println!("  Total runtime: {:?}", resource_estimate.estimated_runtime);
println!("  Success probability: {:.4}", resource_estimate.success_probability);
```

## Summary

PMIR's QEC support enables:

1. **Natural expression** of fault-tolerant algorithms
2. **Automatic lowering** from logical to physical operations
3. **Resource optimization** for different metrics
4. **Integration** with classical decoders
5. **Analysis tools** for threshold and performance
6. **Multiple QEC codes** with easy extension

This comprehensive QEC support makes PMIR suitable for both near-term NISQ algorithms and future large-scale fault-tolerant quantum computing.