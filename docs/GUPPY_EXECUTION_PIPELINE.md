# Guppy Execution Pipeline

## Overview

Guppy **cannot execute code directly**. It must go through a compilation pipeline:

```
Guppy Code → HUGR → LLVM IR → Native Execution
```

## The Complete Pipeline

### 1. **Guppy to HUGR** (Python)
```python
from guppylang import guppy

@guppy
def my_func(x: int) -> int:
    return x + 1

# Compile to HUGR
module_ptr = guppy.compile(my_func)
hugr = module_ptr.package.modules[0]  # In-memory HUGR object
```

### 2. **HUGR to LLVM IR** (Rust via `execute_llvm`)
The `execute_llvm` module (written in Rust with PyO3 bindings) handles:
- Parsing HUGR from serialized bytes
- Applying optimization passes:
  - Monomorphization
  - Dead function removal  
  - Array linearization
  - Constant function inlining
- Compiling to LLVM IR using `hugr::llvm::emit::EmitHugr`

### 3. **LLVM IR to Native Execution**
The LLVM IR is then:
- Compiled to native machine code
- Executed via LLVM's execution engine

## Key Functions in `execute_llvm`

### `compile_module_to_string(pkg_bytes: &[u8]) -> String`
- Takes serialized HUGR package bytes
- Returns LLVM IR as a string

### `run_int_function(pkg_bytes: &[u8], args: Vec<i64>) -> i64`
- Takes serialized HUGR package bytes
- Compiles to LLVM IR
- Creates a wrapper function
- Executes with given arguments
- Returns integer result

### `run_float_function(pkg_bytes: &[u8], args: Vec<f64>) -> f64`
- Same as above but for floating-point functions

## How Guppy Tests Execute Code

From `tests/integration/conftest.py`:
```python
import execute_llvm

# Serialize the HUGR package
package_bytes = module.package.to_bytes()

# Execute the function
result = execute_llvm.run_int_function(package_bytes, args or [])
```

## Key Insights

1. **HUGR is not executable** - it's an intermediate representation
2. **LLVM IR generation is handled by the Rust `hugr` crate**
3. **Execution requires the full compilation pipeline**
4. **The `execute_llvm` module bridges Python and Rust/LLVM**

## Implications for `hugr_sim()` 

A `hugr_sim()` function would need to:

1. Accept a HUGR object (from Guppy or other sources)
2. Serialize it: `hugr_bytes = package.to_bytes()`
3. Use `execute_llvm.compile_module_to_string()` to get LLVM IR
4. Pass the LLVM IR to `llvm_sim()` for execution with noise models

This maintains the separation of concerns where each simulation function handles its specific IR level.