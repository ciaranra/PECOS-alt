# LlvmSim vs qasm_sim API Comparison

## Overview

This document compares the APIs of `LlvmSim` and `qasm_sim()` to ensure feature parity.

## Input Methods

| Feature | qasm_sim | LlvmSim | Notes |
|---------|----------|---------|-------|
| String input | `qasm_sim(qasm_string)` | `.llvm(llvm_ir_string)` | ✅ Different format but equivalent |
| File input | N/A | `.llvm_file(path)` | ✅ LlvmSim has more options |
| HUGR input | N/A | `.hugr(hugr)` | ✅ LlvmSim supports HUGR |
| HUGR bytes | N/A | `.hugr_bytes(bytes)` | ✅ LlvmSim supports serialized HUGR |
| HUGR file | N/A | `.hugr_file(path)` | ✅ LlvmSim supports HUGR files |

## Configuration Methods

| Feature | qasm_sim | LlvmSim | Notes |
|---------|----------|---------|-------|
| Random seed | `.seed(u64)` | `.seed(u64)` | ✅ Identical |
| Worker threads | `.workers(usize)` | `.workers(usize)` | ✅ Identical |
| Auto workers | `.auto_workers()` | ❌ Missing | ⚠️ Should add |
| JSON config | `.config(&serde_json::Value)` | ❌ Missing | ⚠️ Could add if needed |
| Binary string format | `.with_binary_string_format()` | ❌ Missing | ⚠️ Different output format system |

## Noise Model Methods

| Feature | qasm_sim | LlvmSim | Notes |
|---------|----------|---------|-------|
| No noise | Default or `PassThroughNoise` | `.with_no_noise()` | ✅ Equivalent |
| Depolarizing | `.noise(DepolarizingNoise { p })` | `.with_depolarizing_noise(p)` | ✅ Equivalent |
| Custom depolarizing | `.noise(DepolarizingCustomNoise { ... })` | `.with_custom_depolarizing_noise(...)` | ✅ Equivalent |
| Biased depolarizing | `.noise(BiasedDepolarizingNoise { p })` | `.with_biased_depolarizing_noise(p)` | ✅ Equivalent |
| General noise | `.noise(GeneralNoise)` | `.with_general_noise(builder)` | ✅ Equivalent |
| Generic noise setter | `.noise(impl Into<NoiseModelType>)` | `.with_noise_model(NoiseModelConfig)` | ✅ Equivalent |

## Quantum Engine Methods

| Feature | qasm_sim | LlvmSim | Notes |
|---------|----------|---------|-------|
| Set engine type | `.quantum_engine(type)` | `.with_quantum_engine(type)` | ✅ Equivalent |
| State vector shortcut | N/A | `.with_state_vector_engine()` | ✅ LlvmSim has shortcuts |
| Sparse stabilizer shortcut | N/A | `.with_sparse_stabilizer_engine()` | ✅ LlvmSim has shortcuts |

## Execution Methods

| Feature | qasm_sim | LlvmSim | Notes |
|---------|----------|---------|-------|
| Build simulation | `.build()` | `.build()` | ✅ Identical |
| Run directly | `.run(shots)` | `.run(shots)` | ✅ Identical |

## Additional Features

| Feature | qasm_sim | LlvmSim | Notes |
|---------|----------|---------|-------|
| Keep temp files | N/A | `.keep_temp_files(bool)` | ✅ LlvmSim compatibility method |
| Verbose output | N/A | `.verbose(bool)` | ✅ LlvmSim compatibility method |
| Debug output | N/A | `.debug(bool)` | ✅ LlvmSim compatibility method |

## Summary

**Core Features**: ✅ LlvmSim has all core features and more:
- Multiple input formats (LLVM IR, HUGR)
- Same configuration options (seed, workers)
- Equivalent noise model support
- Same quantum engine options with additional shortcuts
- Identical execution API (build/run)

**Missing Features** (minor):
- `.auto_workers()` - Easy to add
- `.config(&serde_json::Value)` - Could add if needed for Python bindings
- `.with_binary_string_format()` - Different output format approach

**Additional Features in LlvmSim**:
- Multiple input source types (files, HUGR)
- Convenience methods for quantum engines
- Compatibility methods for future extensions

**Verdict**: LlvmSim is essentially on par with qasm_sim() and actually provides more flexibility with its multiple input format support.