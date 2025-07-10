# Implementation Plan for pecos-llvm-sim

## Immediate Tasks

### 1. Create Cargo.toml
- [ ] Set up workspace member
- [ ] Add dependencies:
  - `pecos-llvm-runtime` (for `LlvmEngine`)
  - `pecos-hugr-llvm` (for HUGR compilation)
  - `pecos-engines` (for noise models)
  - `pecos-core` (for errors)

### 2. Create Core Types
- [ ] `LlvmSource` enum for different input types
- [ ] `LlvmSimBuilder` struct with builder pattern
- [ ] `LlvmSimulation` struct for built simulation
- [ ] Configuration types matching existing `llvm_sim`

### 3. Move Existing Code
- [ ] Copy `simulation.rs` from `pecos-llvm-runtime`
- [ ] Extract just the simulation parts (not `LlvmEngine`)
- [ ] Update imports and module structure
- [ ] Ensure all tests still pass

### 4. Add HUGR Support
- [ ] Implement HUGR → LLVM compilation in builder
- [ ] Add `hugr()`, `hugr_bytes()`, `hugr_file()` methods
- [ ] Handle compilation errors gracefully

### 5. Update Re-exports
- [ ] Remove `llvm_sim` from `pecos-llvm-runtime`
- [ ] Add appropriate re-exports in new crate
- [ ] Update `pecos-llvm-runtime/src/lib.rs`

### 6. Python Bindings
- [ ] Update `python/pecos-rslib/src/llvm_v3.rs`
- [ ] Import from new crate location
- [ ] Ensure Python API remains unchanged

## Code Structure

```
pecos-llvm-sim/
├── Cargo.toml
├── README.md
├── docs/
│   ├── ARCHITECTURE.md
│   └── IMPLEMENTATION_PLAN.md
└── src/
    ├── lib.rs           # Public API and re-exports
    ├── builder.rs       # LlvmSimBuilder implementation
    ├── source.rs        # LlvmSource enum and conversions
    ├── simulation.rs    # Core simulation logic (moved from pecos-llvm-runtime)
    └── config.rs        # Configuration types

## Key Implementation Details

### LlvmSource Enum
```rust
pub enum LlvmSource {
    LlvmIr(String),
    LlvmFile(PathBuf),
    Hugr(Box<Hugr>),
    HugrBytes(Vec<u8>),
    HugrFile(PathBuf),
}
```

### Builder Methods
```rust
impl LlvmSimBuilder {
    pub fn llvm(ir: impl Into<String>) -> Self
    pub fn llvm_file(path: impl Into<PathBuf>) -> Self
    pub fn hugr(hugr: Hugr) -> Self
    pub fn hugr_bytes(bytes: Vec<u8>) -> Self
    pub fn hugr_file(path: impl Into<PathBuf>) -> Self
    
    // Configuration methods (from existing llvm_sim)
    pub fn seed(mut self, seed: u64) -> Self
    pub fn workers(mut self, workers: usize) -> Self
    pub fn with_noise_model(mut self, noise: NoiseModelConfig) -> Self
    pub fn with_quantum_engine(mut self, engine: QuantumEngineType) -> Self
    
    // Build the simulation
    pub fn build(self) -> Result<LlvmSimulation, PecosError>
}
```

### Compilation Pipeline
1. In `build()`, check the source type
2. If HUGR, use `pecos_hugr_llvm::compile_hugr_bytes_to_string()`
3. If file, read the file first
4. Create `LlvmEngine` from the LLVM IR
5. Wrap in `LlvmSimulation` with configuration

## Testing Plan

1. **Unit Tests**
   - Test each input format
   - Test builder pattern
   - Test error cases

2. **Integration Tests**
   - Compare HUGR and LLVM inputs
   - Test with various circuits
   - Test all configuration options

3. **Migration Tests**
   - Ensure existing tests still pass
   - Test Python bindings work correctly

## Success Criteria

- [ ] All existing `llvm_sim` tests pass
- [ ] HUGR input produces same results as LLVM input
- [ ] Python API remains unchanged
- [ ] Clean separation between crates
- [ ] Documentation is complete