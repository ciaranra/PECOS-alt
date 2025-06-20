# HUGR Compilation Pipeline Structure Proposal

## Current State

The HUGR compilation pipeline in PECOS currently has these components spread across different modules:

### Rust Side
- `crates/pecos-qir/src/hugr/compiler.rs` - Core HUGR->LLVM compilation
- `crates/pecos-qir/src/hugr/python_api.rs` - Python-friendly wrappers
- `python/pecos-rslib/rust/src/hugr_bindings.rs` - PyO3 bindings

### Python Side
- `pecos.frontends.guppy_frontend` - Guppy compilation with multiple backends
- `pecos.execute_llvm` - Execute LLVM compatibility module
- `pecos_rslib.hugr_qir` - Low-level Rust bindings

## Proposed Structure

### 1. Clear Pipeline Stages

```
Guppy Code → HUGR Bytes → LLVM IR/QIR → Execution Results
```

Each transformation should be a distinct, reusable function.

### 2. Rust Side Improvements

Create a clearer module structure in `crates/pecos-qir/src/hugr/`:

```rust
// crates/pecos-qir/src/hugr/pipeline.rs
pub mod pipeline {
    /// Compile HUGR bytes directly to LLVM IR string (no file I/O)
    pub fn hugr_bytes_to_llvm_string(
        hugr_bytes: &[u8],
        config: &CompilerConfig
    ) -> Result<String, CompilerError>;

    /// Compile HUGR file to LLVM IR file
    pub fn hugr_file_to_llvm_file(
        input_path: &Path,
        output_path: &Path,
        config: &CompilerConfig
    ) -> Result<(), CompilerError>;

    /// Execute LLVM IR directly and return results
    pub fn execute_llvm_string(
        llvm_ir: &str,
        shots: u32
    ) -> Result<Vec<BitString>, ExecutionError>;
}
```

### 3. Python Side Structure

The new `compilation_pipeline.py` module provides:

```python
# Stage 1: Guppy -> HUGR
compile_guppy_to_hugr(guppy_function) -> bytes

# Stage 2: HUGR -> LLVM
compile_hugr_to_llvm(hugr_bytes) -> str

# Stage 3: Execute LLVM
execute_llvm(llvm_ir, shots) -> dict

# Convenience functions
compile_guppy_to_llvm(guppy_function) -> str
run_guppy_function(guppy_function, shots) -> dict
```

### 4. Benefits

1. **Clear separation of concerns** - Each stage is independent
2. **Reusability** - Can use any stage independently
3. **Testability** - Can test each transformation separately
4. **Performance** - Avoid unnecessary file I/O by working with strings/bytes
5. **Flexibility** - Easy to add new stages or backends

### 5. Migration Path

1. Keep existing APIs for backward compatibility
2. Implement new pipeline functions alongside existing ones
3. Update documentation to prefer new pipeline
4. Eventually deprecate scattered functions

### 6. Example Usage

```python
from pecos.compilation_pipeline import (
    compile_guppy_to_hugr,
    compile_hugr_to_llvm,
    execute_llvm,
    run_guppy_function
)
from guppylang import guppy

@guppy
def bell_state() -> tuple[bool, bool]:
    q0 = qubit()
    q1 = qubit()
    h(q0)
    cx(q0, q1)
    return measure(q0), measure(q1)

# Option 1: Use the full pipeline
results = run_guppy_function(bell_state, shots=1000)

# Option 2: Use individual stages
hugr_bytes = compile_guppy_to_hugr(bell_state)
llvm_ir = compile_hugr_to_llvm(hugr_bytes)
results = execute_llvm(llvm_ir, shots=1000)

# Option 3: Save intermediate results
with open("bell_state.hugr", "wb") as f:
    f.write(hugr_bytes)
with open("bell_state.ll", "w") as f:
    f.write(llvm_ir)
```

## Implementation Priority

1. **High Priority**: Optimize Rust `compile_hugr_bytes_to_qir_string` to avoid temp files
2. **Medium Priority**: Create the Python `compilation_pipeline` module
3. **Low Priority**: Refactor existing code to use new pipeline

This structure makes the compilation pipeline much clearer and more maintainable.
