# Guppy → HUGR → QIR → PECOS Architecture

## Overview

This document describes the complete quantum programming pipeline from Guppy (quantum language) through HUGR (intermediate representation) to QIR (quantum IR) and finally PECOS execution. This pipeline enables high-level quantum programming with efficient compilation and execution.

## Architecture Diagram

```
┌─────────────┐    ┌──────────────┐    ┌─────────────┐    ┌─────────────┐
│   Guppy     │───▶│     HUGR     │───▶│     QIR     │───▶│    PECOS    │
│ (Python)    │    │ (IR Format)  │    │ (LLVM IR)   │    │ (Execution) │
└─────────────┘    └──────────────┘    └─────────────┘    └─────────────┘
```

## Component Details

### 1. Guppy Language Layer
- **Purpose**: High-level quantum programming in Python
- **Location**: `guppylang/` directory
- **Key Features**:
  - Quantum decorators (`@guppy`)
  - Type-safe quantum operations
  - Classical-quantum hybrid programming
  - Python AST-based compilation

**Example Usage**:
```python
from guppylang import guppy

@guppy
def bell_state() -> None:
    q1, q2 = qubit(), qubit()
    q1 = h(q1)
    q1, q2 = cx(q1, q2)
    measure(q1)
    measure(q2)
```

### 2. HUGR Intermediate Representation
- **Purpose**: Backend-agnostic quantum IR
- **Location**: `hugr/` directory
- **Key Features**:
  - Hierarchical graph representation
  - Type system with quantum/classical distinction
  - Binary serialization format
  - Multiple backend targets

**Compilation**:
```python
compiled = guppy.compile(bell_state)
hugr_bytes = compiled.package.to_bytes()
```

### 3. QIR Generation (LLVM IR)
- **Purpose**: Convert HUGR to executable LLVM IR
- **Location**: `PECOS/crates/pecos-llvm-runtime/src/hugr.rs`
- **Key Features**:
  - HUGR → QIR compilation via Rust
  - PyO3 Python bindings
  - Standard QIR runtime calls
  - Classical computation support

**Integration Points**:
- `compile_hugr_to_qir()` - Core compilation function
- `create_qir_engine_from_hugr()` - Engine creation
- `get_hugr_compilation_info()` - Metadata extraction

### 4. PECOS Execution Engine
- **Purpose**: Execute QIR programs with quantum simulation
- **Location**: `PECOS/crates/pecos-llvm-runtime/src/runtime.rs`
- **Key Features**:
  - QIR runtime implementation
  - Quantum operation primitives
  - Classical-quantum interop
  - Measurement and state management

**Supported Operations**:
- Single-qubit gates: H, X, Y, Z, S, T
- Two-qubit gates: CX, CZ, CCX
- Rotations: RZ, RX, RY
- Measurements and resets

## Integration Architecture

### Python Frontend (GuppyFrontend)
**File**: `PECOS/python/quantum-pecos/src/pecos/frontends/guppy_frontend.py`

```python
class GuppyFrontend:
    def compile_guppy_to_qir(self, guppy_func):
        # 1. Compile Guppy → HUGR
        compiled = guppy.compile(guppy_func)
        hugr_bytes = compiled.package.to_bytes()

        # 2. Compile HUGR → QIR (via Rust)
        qir_string = compile_hugr_to_qir(hugr_bytes)

        # 3. Create execution engine
        engine = create_qir_engine_from_hugr(hugr_bytes)

        return qir_string, engine
```

### Rust Backend Integration
**File**: `PECOS/crates/pecos-llvm-runtime/src/python_api.rs`

```rust
#[pyfunction]
fn compile_hugr_to_qir(hugr_bytes: &[u8]) -> PyResult<String> {
    let hugr = hugr_core::hugr::Hugr::from_bytes(hugr_bytes)?;
    let qir = hugr_to_qir(&hugr)?;
    Ok(qir)
}

#[pyfunction]
fn create_qir_engine_from_hugr(hugr_bytes: &[u8]) -> PyResult<LlvmEngineWrapper> {
    let engine = LlvmEngine::from_hugr_bytes(hugr_bytes)?;
    Ok(LlvmEngineWrapper::new(engine))
}
```

## Current Capabilities

### ✅ Working Features
1. **Complete Pipeline**: Guppy → HUGR → QIR → PECOS execution
2. **Classical Operations**: All arithmetic and control flow
3. **Basic Quantum Gates**: H, X, Y, Z, CX, measurements
4. **Type Safety**: Strong typing throughout the pipeline
5. **Error Handling**: Comprehensive error propagation
6. **Testing Infrastructure**: Automated test suite

### ✅ Integration Points
- PyO3 bindings for Rust backend access
- HUGR binary serialization
- QIR runtime compatibility
- Python package integration

## Next Steps for Expanding Capabilities

### 1. Extended Quantum Gate Set
**Priority**: High
**Effort**: Medium

**Current Gap**: Limited quantum operations
**Goal**: Support full universal gate set

**Implementation**:
```rust
// In pecos-llvm-runtime/src/runtime.rs
#[no_mangle]
pub extern "C" fn __quantum__qis__rx__body(angle: f64, qubit: *mut Qubit) {
    // Implement RX rotation
}

#[no_mangle]
pub extern "C" fn __quantum__qis__ry__body(angle: f64, qubit: *mut Qubit) {
    // Implement RY rotation
}
```

**Required Changes**:
- Add rotation gates to QIR runtime
- Update HUGR → QIR compilation mapping
- Add Guppy language bindings
- Update test suite

### 2. Parameterized Circuits
**Priority**: High
**Effort**: High

**Current Gap**: Static circuit compilation only
**Goal**: Runtime parameter support

**Implementation**:
```python
@guppy
def parameterized_circuit(theta: float) -> None:
    q = qubit()
    q = rx(q, theta)  # Runtime parameter
    measure(q)
```

**Required Changes**:
- HUGR parameter passing mechanisms
- QIR parameter compilation
- Runtime parameter injection
- Type system extensions

### 3. Advanced Quantum Operations
**Priority**: Medium
**Effort**: Medium

**Extensions Needed**:
- Multi-controlled gates
- Quantum arithmetic operations
- Quantum Fourier transforms
- Error correction primitives

**Implementation**:
```rust
// Multi-controlled operations
#[no_mangle]
pub extern "C" fn __quantum__qis__mcx__body(
    controls: *const *mut Qubit,
    control_count: usize,
    target: *mut Qubit
) {
    // Multi-controlled X implementation
}
```

### 4. Quantum Memory Management
**Priority**: Medium
**Effort**: High

**Current Gap**: Simple qubit allocation
**Goal**: Advanced memory management

**Features**:
- Qubit recycling
- Garbage collection
- Memory optimization
- Resource tracking

### 5. Classical-Quantum Interoperability
**Priority**: Medium
**Effort**: Medium

**Enhancements**:
- Conditional quantum operations
- Classical feedback loops
- Mid-circuit measurements
- Adaptive algorithms

### 6. Optimization Passes
**Priority**: Low
**Effort**: High

**Optimizations**:
- Gate fusion and cancellation
- Circuit depth reduction
- Qubit routing optimization
- Compilation optimization

## Development Workflow

### Adding New Quantum Operations

1. **Define in Guppy**:
   ```python
   # In guppylang/std/quantum_functional.py
   def new_gate(qubit: Qubit) -> Qubit:
       # Implementation
   ```

2. **Add HUGR Support**:
   ```rust
   // In hugr/ - extend quantum extension
   ```

3. **Implement QIR Compilation**:
   ```rust
   // In pecos-llvm-runtime/src/hugr.rs
   fn compile_new_gate(node: &Node) -> String {
       // HUGR → QIR compilation
   }
   ```

4. **Add Runtime Support**:
   ```rust
   // In pecos-llvm-runtime/src/runtime.rs
   #[no_mangle]
   pub extern "C" fn __quantum__qis__new_gate__body(qubit: *mut Qubit) {
       // Runtime implementation
   }
   ```

5. **Add Tests**:
   ```python
   # In python/tests/guppy/
   def test_new_gate():
       # Test implementation
   ```

### Testing Strategy
- **Unit Tests**: Each component individually
- **Integration Tests**: Full pipeline testing
- **Regression Tests**: Prevent capability loss
- **Performance Tests**: Benchmark critical paths

## Performance Considerations

### Current Performance Profile
- **Guppy Compilation**: Fast (Python AST processing)
- **HUGR Generation**: Fast (graph construction)
- **QIR Compilation**: Medium (Rust compilation)
- **PECOS Execution**: Fast (optimized simulation)

### Optimization Opportunities
1. **HUGR Caching**: Cache compiled HUGR representations
2. **JIT Compilation**: Just-in-time QIR compilation
3. **Parallel Execution**: Multi-threaded simulation
4. **Memory Optimization**: Reduce allocation overhead

## Troubleshooting Guide

### Common Issues

1. **Import Errors**:
   ```bash
   # Ensure proper build
   cd PECOS && make build
   ```

2. **Type Mismatches**:
   - Check Guppy type annotations
   - Verify HUGR type consistency
   - Validate QIR signatures

3. **Runtime Errors**:
   - Enable debug logging
   - Check qubit lifecycle
   - Verify operation ordering

### Debug Tools
- HUGR visualization: `hugr-cli mermaid`
- QIR inspection: Text editor examination
- Runtime debugging: PECOS logging

## Conclusion

The Guppy → HUGR → QIR → PECOS pipeline provides a complete quantum programming stack with strong foundations for extension. The modular architecture enables independent development of each component while maintaining integration compatibility.

Key strengths:
- Type-safe quantum programming
- Backend flexibility via HUGR
- Efficient execution via PECOS
- Comprehensive testing

The outlined next steps provide a clear roadmap for expanding capabilities while maintaining the architectural integrity of the system.
