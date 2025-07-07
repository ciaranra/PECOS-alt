# PMIR Examples

This document provides comprehensive examples of using PMIR for various quantum computing tasks, from simple circuits to complex fault-tolerant algorithms.

## Table of Contents

1. [Basic Quantum Circuits](#1-basic-quantum-circuits)
2. [Quantum Algorithms](#2-quantum-algorithms)
3. [Classical-Quantum Hybrid Programs](#3-classical-quantum-hybrid-programs)
4. [Fault-Tolerant Circuits](#4-fault-tolerant-circuits)
5. [Optimization Examples](#5-optimization-examples)
6. [Custom Dialects](#6-custom-dialects)
7. [Advanced Patterns](#7-advanced-patterns)

## 1. Basic Quantum Circuits

### 1.1 Bell State Preparation

```rust
use pmir::prelude::*;

// Using the builder API
let bell_circuit = PMIRBuilder::new()
    .with_function("bell_state", Signature::new(vec![], vec![bit_type(); 2]), |f| {
        // Allocate qubits
        let q0 = f.qubit();
        let q1 = f.qubit();
        
        // Create Bell state
        f.h(q0);
        f.cx(q0, q1);
        
        // Measure
        let m0 = f.measure(q0);
        let m1 = f.measure(q1);
        
        f.return_values(vec![m0, m1])
    })
    .build()?;

// Using the circuit builder
let bell_circuit_alt = circuit()
    .qubits(2)
    .h(0)
    .cx(0, 1)
    .measure_all()
    .build();

// Direct PMIR construction
let bell_pmir = Module {
    functions: vec![Function {
        name: "bell_state".into(),
        signature: Signature::new(vec![], vec![bit_type(); 2]),
        regions: vec![Region {
            blocks: vec![Block {
                operations: vec![
                    quantum::h(q0),
                    quantum::cx(q0, q1),
                    quantum::measure(q0, m0),
                    quantum::measure(q1, m1),
                ],
                terminator: Terminator::Return(vec![m0, m1]),
            }],
        }],
    }],
};
```

### 1.2 GHZ State

```rust
// Parameterized GHZ state
fn create_ghz(n: usize) -> Module {
    PMIRBuilder::new()
        .with_function("ghz_state", |f| {
            let qubits = f.qubit_array(n);
            
            // Apply H to first qubit
            f.h(qubits[0]);
            
            // Chain of CNOTs
            for i in 0..n-1 {
                f.cx(qubits[i], qubits[i+1]);
            }
            
            // Measure all
            let measurements = qubits.iter()
                .map(|&q| f.measure(q))
                .collect();
                
            f.return_values(measurements)
        })
        .build()
        .unwrap()
}
```

### 1.3 Quantum Fourier Transform

```rust
fn qft_circuit(n: usize) -> Module {
    PMIRBuilder::new()
        .with_function("qft", |f| {
            let qubits = f.qubit_array(n);
            
            for i in 0..n {
                // Hadamard
                f.h(qubits[i]);
                
                // Controlled rotations
                for j in i+1..n {
                    let angle = PI / (1 << (j - i));
                    f.controlled_phase(qubits[j], qubits[i], angle);
                }
            }
            
            // Swap qubits
            for i in 0..n/2 {
                f.swap(qubits[i], qubits[n-1-i]);
            }
        })
        .build()
        .unwrap()
}
```

## 2. Quantum Algorithms

### 2.1 Grover's Algorithm

```rust
fn grover_search(n_qubits: usize, marked_item: usize) -> Module {
    PMIRBuilder::new()
        .with_function("grover", |f| {
            let qubits = f.qubit_array(n_qubits);
            let ancilla = f.qubit();
            
            // Initialize superposition
            for &q in &qubits {
                f.h(q);
            }
            f.x(ancilla);
            f.h(ancilla);
            
            // Grover iterations
            let n_iterations = ((PI/4.0) * (2.0_f64.powi(n_qubits as i32)).sqrt()) as usize;
            
            for _ in 0..n_iterations {
                // Oracle
                f.call_function("oracle", vec![qubits.clone(), vec![ancilla], vec![marked_item.into()]]);
                
                // Diffusion operator
                for &q in &qubits {
                    f.h(q);
                    f.x(q);
                }
                
                f.multi_controlled_z(qubits.clone());
                
                for &q in &qubits {
                    f.x(q);
                    f.h(q);
                }
            }
            
            // Measure result
            let results = qubits.iter().map(|&q| f.measure(q)).collect();
            f.return_values(results)
        })
        .build()
        .unwrap()
}
```

### 2.2 Variational Quantum Eigensolver (VQE)

```rust
fn vqe_circuit(n_qubits: usize, n_layers: usize) -> Module {
    PMIRBuilder::new()
        .with_function("vqe_ansatz", 
            Signature::new(
                vec![array_type(float_type(), n_qubits * n_layers * 3)], // parameters
                vec![float_type()] // expectation value
            ),
            |f| {
                let params = f.arg(0);
                let qubits = f.qubit_array(n_qubits);
                
                // Initial state preparation
                for &q in &qubits {
                    f.h(q);
                }
                
                // Parameterized layers
                let mut param_idx = 0;
                for layer in 0..n_layers {
                    // Single qubit rotations
                    for i in 0..n_qubits {
                        let rx_angle = f.array_get(params, param_idx);
                        let ry_angle = f.array_get(params, param_idx + 1);
                        let rz_angle = f.array_get(params, param_idx + 2);
                        
                        f.rx(qubits[i], rx_angle);
                        f.ry(qubits[i], ry_angle);
                        f.rz(qubits[i], rz_angle);
                        
                        param_idx += 3;
                    }
                    
                    // Entangling layer
                    for i in 0..n_qubits-1 {
                        f.cx(qubits[i], qubits[i+1]);
                    }
                    if n_qubits > 2 {
                        f.cx(qubits[n_qubits-1], qubits[0]);
                    }
                }
                
                // Measure expectation value of Hamiltonian
                let expectation = f.call_function("measure_hamiltonian", vec![qubits]);
                f.return_value(expectation)
            }
        )
        .build()
        .unwrap()
}
```

### 2.3 Quantum Phase Estimation

```rust
fn qpe_circuit(n_precision_qubits: usize) -> Module {
    PMIRBuilder::new()
        .with_function("qpe", |f| {
            // Precision qubits for storing the phase
            let precision = f.qubit_array(n_precision_qubits);
            // Target qubit in eigenstate
            let target = f.qubit();
            
            // Initialize precision qubits in superposition
            for &q in &precision {
                f.h(q);
            }
            
            // Controlled unitary operations
            for (i, &control) in precision.iter().enumerate() {
                let power = 1 << (n_precision_qubits - 1 - i);
                for _ in 0..power {
                    f.controlled_unitary(control, target, "U");
                }
            }
            
            // Inverse QFT on precision qubits
            f.call_function("inverse_qft", vec![precision.clone()]);
            
            // Measure precision qubits
            let phase_bits = precision.iter()
                .map(|&q| f.measure(q))
                .collect::<Vec<_>>();
                
            // Convert to phase value
            let phase = f.call_function("bits_to_phase", vec![phase_bits]);
            f.return_value(phase)
        })
        .build()
        .unwrap()
}
```

## 3. Classical-Quantum Hybrid Programs

### 3.1 QAOA Circuit

```rust
fn qaoa_maxcut(graph: Graph, p: usize) -> Module {
    PMIRBuilder::new()
        .with_function("qaoa",
            Signature::new(
                vec![array_type(float_type(), 2*p)], // beta and gamma parameters
                vec![float_type()] // expectation value
            ),
            |f| {
                let params = f.arg(0);
                let n_qubits = graph.num_vertices();
                let qubits = f.qubit_array(n_qubits);
                
                // Initial state: uniform superposition
                for &q in &qubits {
                    f.h(q);
                }
                
                // QAOA layers
                for layer in 0..p {
                    let gamma = f.array_get(params, 2*layer);
                    let beta = f.array_get(params, 2*layer + 1);
                    
                    // Problem Hamiltonian (MaxCut)
                    for edge in graph.edges() {
                        f.rzz(qubits[edge.0], qubits[edge.1], gamma);
                    }
                    
                    // Mixer Hamiltonian
                    for &q in &qubits {
                        f.rx(q, f.mul(beta, f.constant(2.0)));
                    }
                }
                
                // Measure and compute expectation
                let measurements = qubits.iter()
                    .map(|&q| f.measure(q))
                    .collect::<Vec<_>>();
                    
                let expectation = f.call_function("compute_cut_value", 
                    vec![measurements, graph.to_value()]);
                    
                f.return_value(expectation)
            }
        )
        .build()
        .unwrap()
}
```

### 3.2 Quantum Machine Learning

```rust
fn quantum_kernel_circuit(n_features: usize) -> Module {
    PMIRBuilder::new()
        .with_function("quantum_kernel",
            Signature::new(
                vec![
                    array_type(float_type(), n_features), // x
                    array_type(float_type(), n_features), // y
                ],
                vec![float_type()] // kernel value
            ),
            |f| {
                let x = f.arg(0);
                let y = f.arg(1);
                let qubits = f.qubit_array(n_features);
                
                // Feature map for x
                f.call_function("feature_map", vec![qubits.clone(), x]);
                
                // Inverse feature map for y
                f.call_function("inverse_feature_map", vec![qubits.clone(), y]);
                
                // Measure all qubits
                let measurements = qubits.iter()
                    .map(|&q| f.measure(q))
                    .collect::<Vec<_>>();
                
                // Compute kernel as probability of all zeros
                let kernel = f.call_function("compute_kernel_value", vec![measurements]);
                f.return_value(kernel)
            }
        )
        .with_function("feature_map", |f| {
            let qubits = f.arg(0);
            let features = f.arg(1);
            
            // Amplitude encoding
            for i in 0..n_features {
                let q = f.array_get(qubits, i);
                let feature = f.array_get(features, i);
                
                f.ry(q, feature);
                
                // Entangling layer
                if i < n_features - 1 {
                    let q_next = f.array_get(qubits, i + 1);
                    f.cx(q, q_next);
                }
            }
        })
        .build()
        .unwrap()
}
```

## 4. Fault-Tolerant Circuits

### 4.1 Surface Code Operations

```rust
fn surface_code_logical_gates() -> Module {
    PMIRBuilder::new()
        .with_qec_dialect()
        .with_function("surface_code_demo", |f| {
            // Initialize logical qubits with surface code
            let logical1 = f.qec_init_logical(
                ErrorCorrectionCode::Surface { distance: 17 },
                LogicalState::Zero
            );
            let logical2 = f.qec_init_logical(
                ErrorCorrectionCode::Surface { distance: 17 },
                LogicalState::Plus
            );
            
            // Logical CNOT via lattice surgery
            f.qec_logical_cnot_surgery(logical1, logical2);
            
            // Syndrome extraction round
            let syndrome = f.qec_extract_syndrome(vec![logical1, logical2]);
            
            // Decode and correct
            let correction = f.qec_decode_mwpm(syndrome);
            f.qec_apply_correction(vec![logical1, logical2], correction);
            
            // Logical measurement
            let result1 = f.qec_logical_measure(logical1, PauliBasis::Z);
            let result2 = f.qec_logical_measure(logical2, PauliBasis::X);
            
            f.return_values(vec![result1, result2])
        })
        .build()
        .unwrap()
}
```

### 4.2 Magic State Distillation

```rust
fn magic_state_factory() -> Module {
    PMIRBuilder::new()
        .with_qec_dialect()
        .with_function("distill_t_state", |f| {
            // Prepare 15 noisy T states
            let noisy_states = (0..15)
                .map(|_| f.qec_prepare_noisy_magic(MagicStateType::T, 0.99))
                .collect::<Vec<_>>();
                
            // 15-to-1 distillation protocol
            let distilled = f.qec_distill_15_to_1(noisy_states);
            
            // Verify fidelity
            f.qec_assert_fidelity(distilled, 0.9999);
            
            f.return_value(distilled)
        })
        .with_function("t_gate_via_magic", |f| {
            let logical = f.arg(0); // Logical qubit
            
            // Get magic state from factory
            let magic = f.call_function("distill_t_state", vec![]);
            
            // Implement T gate via state injection
            f.qec_inject_magic_state(logical, magic);
            
            f.return_value(logical)
        })
        .build()
        .unwrap()
}
```

### 4.3 Fault-Tolerant Arithmetic

```rust
fn fault_tolerant_adder(n_bits: usize) -> Module {
    PMIRBuilder::new()
        .with_qec_dialect()
        .with_function("ft_ripple_carry_adder",
            Signature::new(
                vec![
                    array_type(logical_qubit_type(), n_bits), // a
                    array_type(logical_qubit_type(), n_bits), // b
                ],
                vec![array_type(logical_qubit_type(), n_bits + 1)] // sum with carry
            ),
            |f| {
                let a = f.arg(0);
                let b = f.arg(1);
                let n = n_bits;
                
                // Allocate ancilla logical qubits
                let carry = f.qec_init_logical_array(n + 1, LogicalState::Zero);
                
                // Ripple carry addition with error correction
                for i in 0..n {
                    let ai = f.array_get(a, i);
                    let bi = f.array_get(b, i);
                    let ci = f.array_get(carry, i);
                    let ci_plus = f.array_get(carry, i + 1);
                    
                    // Fault-tolerant full adder
                    f.qec_logical_cx(bi, ci_plus);
                    f.qec_logical_cx(ai, ci_plus);
                    f.qec_logical_ccx(ai, bi, ci_plus);
                    f.qec_logical_cx(ai, bi);
                    f.qec_logical_cx(ci, bi);
                    f.qec_logical_ccx(ci, ai, ci_plus);
                    
                    // Syndrome extraction every few gates
                    if i % 3 == 2 {
                        let syndrome = f.qec_extract_syndrome_all();
                        let correction = f.qec_decode(syndrome);
                        f.qec_apply_correction_all(correction);
                    }
                }
                
                // Final syndrome round
                f.qec_full_error_correction();
                
                // Result is in b register and final carry
                let mut result = Vec::new();
                for i in 0..n {
                    result.push(f.array_get(b, i));
                }
                result.push(f.array_get(carry, n));
                
                f.return_value(f.make_array(result))
            }
        )
        .build()
        .unwrap()
}
```

## 5. Optimization Examples

### 5.1 Gate Fusion

```rust
// Before optimization
let unoptimized = Module::parse(r#"
    func @consecutive_rotations(%q: !quantum.qubit) {
        %q1 = quantum.rz(%q, 0.5) : !quantum.qubit
        %q2 = quantum.rz(%q1, 0.3) : !quantum.qubit
        %q3 = quantum.rx(%q2, 1.0) : !quantum.qubit
        %q4 = quantum.rx(%q3, 0.5) : !quantum.qubit
        return %q4 : !quantum.qubit
    }
"#)?;

// Apply optimization pass
let optimizer = PassManager::new()
    .add_pass(GateFusionPass::new())
    .add_pass(ConstantFoldingPass::new());
    
let optimized = optimizer.run(unoptimized)?;

// After optimization
// func @consecutive_rotations(%q: !quantum.qubit) {
//     %q1 = quantum.rz(%q, 0.8) : !quantum.qubit   // Fused RZ
//     %q2 = quantum.rx(%q1, 1.5) : !quantum.qubit  // Fused RX
//     return %q2 : !quantum.qubit
// }
```

### 5.2 Circuit Routing

```rust
fn route_circuit_for_hardware(circuit: Module, hardware: HardwareTopology) -> Module {
    let router = CircuitRouter::new(hardware);
    
    PMIRBuilder::from_module(circuit)
        .apply_pass(router)
        .apply_pass(SwapOptimizer::new())
        .build()
        .unwrap()
}

// Example: Route for linear topology
let hardware = HardwareTopology::Linear(5);
let routed = route_circuit_for_hardware(original_circuit, hardware);
```

### 5.3 Measurement Optimization

```rust
// Optimize measurement scheduling
let measurement_optimizer = MeasurementScheduler::new()
    .with_strategy(SchedulingStrategy::MinimizeDepth)
    .with_commutation_analysis(true);

let optimized = PMIRBuilder::from_module(circuit)
    .apply_pass(measurement_optimizer)
    .build()?;
```

## 6. Custom Dialects

### 6.1 Pulse-Level Control

```rust
// Define custom pulse dialect
struct PulseDialect;

impl Dialect for PulseDialect {
    fn name(&self) -> &str { "pulse" }
    
    fn initialize(&self, registry: &mut DialectRegistry) {
        // Pulse operations
        registry.register_op("pulse.gaussian", gaussian_pulse_op());
        registry.register_op("pulse.drag", drag_pulse_op());
        registry.register_op("pulse.cr", cross_resonance_op());
        
        // Pulse types
        registry.register_type("pulse.waveform", waveform_type());
        registry.register_type("pulse.channel", channel_type());
    }
}

// Use pulse dialect
let pulse_circuit = PMIRBuilder::new()
    .add_dialect(PulseDialect)
    .with_function("custom_gate", |f| {
        let q0_drive = f.pulse_channel("q0_drive");
        let q0_control = f.pulse_channel("q0_control");
        
        // Custom X gate implementation
        let x_pulse = f.pulse_gaussian(
            amplitude: 0.5,
            sigma: 20.0,
            duration: 100.0,
        );
        
        f.pulse_play(x_pulse, q0_drive);
        
        // Custom CR gate
        let cr_pulse = f.pulse_cr(
            amplitude: 0.3,
            duration: 200.0,
            phase: PI/4.0,
        );
        
        f.pulse_play(cr_pulse, q0_control);
    })
    .build()?;
```

### 6.2 Quantum Chemistry

```rust
// Chemistry-specific dialect
struct ChemistryDialect;

impl Dialect for ChemistryDialect {
    fn initialize(&self, registry: &mut DialectRegistry) {
        registry.register_op("chem.prepare_slater", slater_determinant_op());
        registry.register_op("chem.jordan_wigner", jordan_wigner_op());
        registry.register_op("chem.uccsd", uccsd_ansatz_op());
    }
}

let chemistry_circuit = PMIRBuilder::new()
    .add_dialect(ChemistryDialect)
    .with_function("h2_ground_state", |f| {
        let qubits = f.qubit_array(4); // 4 spin orbitals
        
        // Prepare Hartree-Fock state |1100>
        f.chem_prepare_slater(qubits, vec![0, 1]);
        
        // UCCSD ansatz
        let params = f.arg(0); // Variational parameters
        f.chem_uccsd(qubits, params);
        
        // Measure energy
        let energy = f.chem_measure_energy(qubits, "h2_hamiltonian");
        f.return_value(energy)
    })
    .build()?;
```

## 7. Advanced Patterns

### 7.1 Dynamic Circuits

```rust
fn teleportation_with_feedforward() -> Module {
    PMIRBuilder::new()
        .with_function("quantum_teleport", |f| {
            // Alice's qubits
            let alice_data = f.qubit();
            let alice_bell = f.qubit();
            
            // Bob's qubit
            let bob_bell = f.qubit();
            
            // Create Bell pair between Alice and Bob
            f.h(alice_bell);
            f.cx(alice_bell, bob_bell);
            
            // Alice's Bell measurement
            f.cx(alice_data, alice_bell);
            f.h(alice_data);
            let m1 = f.measure(alice_data);
            let m2 = f.measure(alice_bell);
            
            // Classical communication and feedforward
            f.if_then(m2, || {
                f.x(bob_bell);
            });
            
            f.if_then(m1, || {
                f.z(bob_bell);
            });
            
            // Bob now has the teleported state
            f.return_value(bob_bell)
        })
        .build()
        .unwrap()
}
```

### 7.2 Quantum Error Mitigation

```rust
fn zero_noise_extrapolation() -> Module {
    PMIRBuilder::new()
        .with_function("zne_circuit",
            Signature::new(
                vec![float_type()], // noise scaling factor
                vec![float_type()], // expectation value
            ),
            |f| {
                let scale_factor = f.arg(0);
                let qubits = f.qubit_array(10);
                
                // Original circuit
                f.call_function("my_algorithm", vec![qubits.clone()]);
                
                // Insert scaled noise via identity gates
                let n_identities = f.mul(scale_factor, f.constant(10.0));
                f.for_loop(f.constant(0), n_identities, f.constant(1.0), |loop_builder, i| {
                    let q_idx = loop_builder.mod(i, f.constant(10));
                    let q = loop_builder.array_get(qubits, q_idx);
                    
                    // Identity implemented as two Pauli gates (adds noise)
                    loop_builder.x(q);
                    loop_builder.x(q);
                });
                
                // Measure expectation
                let expectation = f.measure_expectation(qubits, "observable");
                f.return_value(expectation)
            }
        )
        .build()
        .unwrap()
}
```

### 7.3 Adaptive Algorithms

```rust
fn adaptive_vqe() -> Module {
    PMIRBuilder::new()
        .with_function("adaptive_vqe_step", |f| {
            let current_params = f.arg(0);
            let qubits = f.qubit_array(4);
            
            // Run current ansatz
            f.call_function("vqe_ansatz", vec![qubits.clone(), current_params]);
            
            // Measure commutators to find next operator
            let commutators = f.create_array(10);
            for i in 0..10 {
                let comm_val = f.call_function("measure_commutator", 
                    vec![qubits.clone(), f.constant(i)]);
                f.array_set(commutators, i, comm_val);
            }
            
            // Find largest gradient
            let (max_idx, max_val) = f.call_function("argmax", vec![commutators]);
            
            // Decide whether to add operator
            f.if_then_else(
                f.greater_than(max_val, f.constant(0.01)),
                || {
                    // Add new parameter
                    let new_params = f.array_append(current_params, f.constant(0.0));
                    f.return_value(new_params)
                },
                || {
                    // Convergence reached
                    f.return_value(current_params)
                }
            )
        })
        .build()
        .unwrap()
}
```

### 7.4 Quantum Machine Compilation

```rust
// Compile high-level algorithm to hardware-specific implementation
fn compile_for_hardware(algorithm: Module, hardware: Hardware) -> Module {
    let pipeline = CompilationPipeline::new()
        // High-level optimizations
        .add_pass(AlgorithmicOptimizer::new())
        
        // Decompose to native gates
        .add_pass(GateDecomposer::new(hardware.native_gates()))
        
        // Route for topology
        .add_pass(CircuitRouter::new(hardware.topology()))
        
        // Hardware-specific optimizations
        .add_pass(hardware.optimization_pass())
        
        // Error mitigation
        .add_pass(ErrorMitigationPass::new(hardware.error_model()))
        
        // Pulse optimization
        .add_pass(PulseOptimizer::new(hardware.pulse_library()));
        
    pipeline.compile(algorithm).unwrap()
}
```

## Running Examples

### Using the Interpreter

```rust
use pmir::interpreter::Interpreter;

let module = create_bell_circuit();
let mut interpreter = Interpreter::new();
let result = interpreter.execute_module(&module)?;
println!("Bell state measurement: {:?}", result);
```

### Generating Rust Code

```rust
use pmir::codegen::RustCodegen;

let module = create_vqe_circuit(4, 2);
let codegen = RustCodegen::new();
let rust_code = codegen.generate(&module)?;

// Compile and run
std::fs::write("vqe.rs", rust_code)?;
std::process::Command::new("rustc")
    .args(&["vqe.rs", "-o", "vqe"])
    .status()?;
```

### Exporting to MLIR

```rust
let module = create_grover_circuit(5);
let mlir_text = module.to_mlir_text();
println!("{}", mlir_text);

// Can be further compiled with MLIR tools
std::fs::write("grover.mlir", mlir_text)?;
```

## Best Practices

1. **Use builders for construction** - They provide type safety and ergonomic APIs
2. **Apply optimization passes** - Even simple passes can significantly improve performance
3. **Validate early** - Use verification passes to catch errors during construction
4. **Profile your circuits** - Use the built-in profiling to identify bottlenecks
5. **Leverage parallelism** - Mark independent operations for parallel execution
6. **Consider hardware constraints** - Use routing passes when targeting specific devices
7. **Test with small examples** - Verify correctness before scaling up