# LLVM-IR Conventions

## Current Status

PECOS uses the HUGR convention for LLVM-IR quantum operations. This convention uses integer-based quantum operations that integrate directly with the PECOS runtime.

### HUGR Convention Details

- **Function signatures**: `void @__quantum__qis__h__body(i64)`
- **Measurements**: Return i32: `i32 @__quantum__qis__m__body(i64, i64)`
- **Entry points**: Return i1 for main functions
- **Qubit representation**: Qubits are represented as i64 integers (indices)
- **Result representation**: Results are represented as i64 integers

### Example LLVM-IR

```llvm
; Quantum gate operations use i64 parameters
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__cnot__body(i64, i64)
declare i32 @__quantum__qis__m__body(i64, i64)

; Resource allocation
declare i64 @__quantum__rt__qubit_allocate()
declare i64 @__quantum__rt__result_allocate()

; Example quantum function
define i1 @bell_state() {
entry:
  %q0 = call i64 @__quantum__rt__qubit_allocate()
  %q1 = call i64 @__quantum__rt__qubit_allocate()
  
  ; Apply Hadamard to first qubit
  call void @__quantum__qis__h__body(i64 %q0)
  
  ; Apply CNOT
  call void @__quantum__qis__cnot__body(i64 %q0, i64 %q1)
  
  ; Measure both qubits
  %r0 = call i64 @__quantum__rt__result_allocate()
  %r1 = call i64 @__quantum__rt__result_allocate()
  %m0 = call i32 @__quantum__qis__m__body(i64 %q0, i64 %r0)
  %m1 = call i32 @__quantum__qis__m__body(i64 %q1, i64 %r1)
  
  ; Return result
  %result = icmp eq i32 %m0, %m1
  ret i1 %result
}
```

## Usage

When using PECOS with Guppy or HUGR:

```python
from pecos import run_guppy

# Execute quantum function - automatically uses HUGR convention
results = run_guppy(quantum_function, shots=1000)
```

For direct LLVM-IR compilation:

```python
from pecos_rslib import compile_hugr_to_llvm_rust

# Compile HUGR to LLVM-IR (HUGR convention)
llvm_ir = compile_hugr_to_llvm_rust(hugr_bytes)
```

## Runtime Integration

The PECOS runtime expects HUGR convention LLVM-IR:
- Qubits are managed as indices in the quantum state vector
- Gates operate directly on these indices
- Measurements return integer results (0 or 1)
- All quantum operations are implemented in the `pecos-qir` runtime module