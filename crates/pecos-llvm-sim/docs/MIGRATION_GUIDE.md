# Migration Guide: Moving to pecos-llvm-sim

This guide helps you migrate from using `llvm_sim` in `pecos-llvm-runtime` to the new `pecos-llvm-sim` crate.

## What Changed?

The simulation functionality has been moved from `pecos-llvm-runtime` to a dedicated `pecos-llvm-sim` crate. This provides:
- Support for multiple input formats (LLVM IR, HUGR, files)
- Cleaner separation of concerns
- Better architectural alignment

## Migration Steps

### 1. Update Your Dependencies

#### Rust
```toml
# Old
[dependencies]
pecos-llvm-runtime = "0.1"

# New
[dependencies]
pecos-llvm-sim = "0.1"
```

#### Python
The Python imports remain the same for now, but will change in a future version:
```python
# Currently works (deprecated)
from pecos_rslib import llvm_sim

# Future (recommended)
from pecos_rslib.llvm_sim import llvm_sim
```

### 2. Update Your Imports

#### Rust
```rust
// Old
use pecos_llvm_runtime::{llvm_sim, LlvmSimBuilder, NoiseModelConfig};

// New
use pecos_llvm_sim::{llvm_sim, LlvmSim, NoiseModelConfig};
```

### 3. API Changes

The API remains largely the same, but with new capabilities:

#### Existing Usage (Still Works)
```rust
// From LLVM IR string
let results = llvm_sim(llvm_ir)
    .seed(42)
    .workers(8)
    .with_depolarizing_noise(0.01)
    .run(1000)?;
```

#### New Capabilities
```rust
// From HUGR
use pecos_llvm_sim::LlvmSim;
let results = LlvmSim::new().hugr(hugr)
    .seed(42)
    .run(1000)?;

// From files
let results = LlvmSim::new().llvm_file("circuit.ll")
    .run(1000)?;

let results = LlvmSim::new().hugr_file("circuit.hugr")
    .run(1000)?;
```

## Benefits of Migration

1. **Future-proof**: The old location is deprecated and will be removed
2. **New features**: Support for HUGR input formats
3. **Better architecture**: Cleaner separation between compilation and execution
4. **Consistent API**: Same builder pattern across all input types

## Timeline

- **Current**: Both locations work, old location shows deprecation warning
- **Next minor version**: Old location removed from `pecos-llvm-runtime`
- **Recommendation**: Migrate as soon as possible to avoid breaking changes

## Need Help?

If you encounter issues during migration:
1. Check the examples in `pecos-llvm-sim/examples/`
2. Review the API documentation
3. File an issue on the PECOS repository