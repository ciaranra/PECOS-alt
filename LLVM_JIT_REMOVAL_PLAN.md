# LLVM JIT Removal Plan

## Phase 1: Conditional Disable (Immediate)
1. **Add feature flag**: `llvm-jit` (disabled by default)
2. **Wrap problematic code** in `#[cfg(feature = "llvm-jit")]`
3. **Provide alternative implementations** for disabled functions
4. **Update tests** to use alternative execution paths

## Phase 2: Alternative Path Development (Ongoing)
1. **Enhance PMIR pipeline** to compile directly to ByteMessage
2. **Improve QASM frontend** for Guppy integration  
3. **Develop HUGR→ByteMessage** direct compilation
4. **Add comprehensive testing** for alternative paths

## Phase 3: Future Re-enablement (Optional)
1. **Fix LLVM JIT issues** when time permits
2. **Re-enable feature flag** for advanced use cases
3. **Maintain both paths** for flexibility

## Files to Modify:

### Cargo.toml changes:
```toml
[features]
default = ["qasm", "phir", "pmir"]
llvm-jit = ["llvm-sys", "inkwell"]  # Disable by default
qasm = []
phir = []
pmir = []
```

### Code changes:
- Wrap `execute_qir()` in `#[cfg(feature = "llvm-jit")]`
- Provide stub implementation when disabled
- Update CLI to show feature availability
- Modify tests to skip when feature disabled

## Benefits:
- ✅ **Immediate relief** from hanging/segfaulting tests
- ✅ **Preserve all working functionality**
- ✅ **Clean path forward** for development
- ✅ **Easy to re-enable** when issues are fixed
- ✅ **Alternative paths become primary** (better architecture)

## Minimal disruption:
- Most users won't notice (alternatives work better anyway)
- Development can continue on all other fronts
- LLVM JIT becomes optional enhancement rather than core dependency