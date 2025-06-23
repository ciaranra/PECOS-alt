# LLVM-IR Conventions

## Current Status (2025-01-22)

PECOS supports two LLVM-IR conventions for quantum operations:

1. **HUGR Convention** (`llvm_convention="hugr"`) - Default, uses integer-based quantum operations
   - Function signatures: `void @__quantum__qis__h__body(i64)`
   - Measurements return i32: `i32 @__quantum__qis__m__body(i64, i64)`
   - Entry points return i1
   - Works with current PECOS runtime

2. **QIR Convention** (`llvm_convention="qir"`) - Microsoft QIR standard with opaque pointer types
   - Declares opaque types: `%Qubit = type opaque`, `%Result = type opaque`
   - Function signatures: `void @__quantum__qis__h__body(%Qubit*)`
   - Measurements return void: `void @__quantum__qis__m__body(%Qubit*, %Result*)`
   - Entry points return void
   - Currently causes runtime errors with PECOS

## Why HUGR Convention is Default

While we've implemented full support for generating QIR convention LLVM-IR (with all quantum gates including Rx, Ry, Toffoli, etc.), the PECOS runtime currently expects HUGR-style integer-based operations. Attempting to execute QIR convention code results in "index out of bounds" errors during quantum operations.

The runtime converts pointer values directly to indices (`qubit as usize`), which works for HUGR convention but causes issues with QIR's pointer-based approach.

## Future Work

To fully support QIR convention, the PECOS runtime needs updates to:
1. Handle opaque pointer types properly
2. Support proper qubit allocation/deallocation
3. Handle void-returning measurements
4. Support QIR entry point conventions

## Usage

To generate QIR convention LLVM-IR (for compatibility with Microsoft QIR tools):
```python
from pecos import run_guppy
# Generate QIR but note it won't execute on PECOS
results = run_guppy(quantum_function, llvm_convention="qir")

# Or just compile without executing:
from pecos.compilation_pipeline import compile_guppy_to_llvm
qir_string = compile_guppy_to_llvm(quantum_function, llvm_convention="qir")
```

For execution with PECOS (default):
```python
from pecos import run_guppy
results = run_guppy(quantum_function)  # Uses HUGR convention by default
```