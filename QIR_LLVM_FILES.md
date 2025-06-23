# QIR and HUGR LLVM Files in PECOS

This document describes the LLVM-IR files in this repository and their conventions.

## Proper QIR Convention Files

These files follow the official Microsoft QIR specification:

### `examples/qir/bell.ll`
- **Convention**: True QIR 
- **Format**: Uses `%Qubit*` and `%Result*` opaque pointer types
- **Qubit Representation**: `%Qubit* null` for qubit 0, `inttoptr (i64 1 to %Qubit*)` for qubit 1
- **Function Signatures**: `void @__quantum__qis__h__body(%Qubit*)`
- **Usage**: `execute_qir(file, shots, seed, None, None, llvm_convention='qir')`

### `examples/qir/qprog.ll`
- **Convention**: True QIR
- **Format**: Uses `%Qubit*` and `%Result*` opaque pointer types
- **Qubit Representation**: `%Qubit* null` for qubit 0, `inttoptr (i64 1 to %Qubit*)` for qubit 1
- **Function Signatures**: `void @__quantum__qis__rz__body(double, %Qubit*)`
- **Usage**: `execute_qir(file, shots, seed, None, None, llvm_convention='qir')`

### `test_null_name.ll`
- **Convention**: True QIR (test case)
- **Format**: Uses `%Qubit*` and `%Result*` opaque pointer types
- **Purpose**: Tests null string handling in result recording
- **Usage**: `execute_qir(file, shots, seed, None, None, llvm_convention='qir')`

## HUGR Convention Files

These files use integer-based quantum operations:

### `examples/bell_final.ll`
- **Convention**: HUGR
- **Format**: Uses integer types directly (`i64`, `i16`)
- **Qubit Representation**: Direct integers (0, 1, 2, etc.)
- **Function Signatures**: `void @__quantum__qis__h__body(i64)`
- **Usage**: `execute_qir(file, shots, seed, None, None, llvm_convention='hugr')`

## Removed Non-Compliant Files

These files were removed because they mixed conventions or used non-standard formats:

- `debug_execution_qir.ll` - Used `i8*` instead of `%Qubit*` but claimed to be QIR
- `python/test_output.ll` - Mixed integer and pointer approaches
- `test_output.ll` - Used `i8*` instead of `%Qubit*` but claimed to be QIR

## Convention Guidelines

### Use QIR Convention When:
- You have existing QIR code from Microsoft Q# compiler
- You need compatibility with other QIR tools
- You want to follow the official QIR specification
- You're using the PMIR backend (it only supports pointer-based)

### Use HUGR Convention When:
- You're compiling from Guppy/HUGR with the Rust backend
- You want simpler integer-based representations
- You're working with PECOS-native quantum programs
- Note: PMIR backend does NOT support HUGR convention

## Runtime Implementation

Both conventions are supported by the PECOS QIR runtime:

- **QIR**: Pointer values are interpreted as direct qubit indices via `inttoptr`
- **HUGR**: Integers are used directly as qubit indices
- Both use the same underlying quantum operations but with different interfaces

## Backend Support

- **Rust Backend** (`backend="rust"`): Supports both QIR and HUGR conventions
- **PMIR Backend** (`backend="external"`): Only supports pointer-based convention (similar to QIR)
  - PMIR generates `i8*` for qubits instead of integers
  - The runtime automatically uses QIR convention for PMIR-generated code