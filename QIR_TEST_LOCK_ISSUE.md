# QIR Test Lock Issue - Technical Debt

## Summary

The QIR tests currently use a file-based lock mechanism to prevent concurrent execution. This is a workaround for underlying architectural issues that should be properly addressed.

## Current Problem

QIR tests in `pecos-cli` cannot run concurrently due to:

1. **Global State Issues**
   - Global interactive callback: `static INTERACTIVE_CALLBACK` in `runtime.rs`
   - Global runtime registry: `static RUNTIME_REGISTRY` 
   - Global build mutex: `static BUILD_MUTEX`

2. **LLVM Runtime Cleanup Issues**
   - Known segfaults during cleanup in multi-threaded environments
   - LLVM JIT feature disabled due to pytest conflicts
   - Race conditions during library unloading

3. **Resource Contention**
   - Multiple tests loading LLVM libraries simultaneously
   - File system races ("Text file busy" errors)

## Current Workaround

File-based lock in `qir_test_lock.rs` that:
- Forces tests to run sequentially
- Uses `target/pecos_qir_test.lock` as coordination file
- Times out after 60 seconds if lock cannot be acquired

## Proper Solutions

### Short Term
1. Use `serial_test` crate instead of custom lock implementation
2. Add `#[serial]` attribute to QIR tests
3. Document why tests must be serial

### Long Term
1. **Eliminate Global State**
   ```rust
   // Instead of global static
   struct QirContext {
       callback: Option<InteractiveCallback>,
       registry: RuntimeRegistry,
   }
   ```

2. **Fix LLVM Cleanup**
   - Proper RAII patterns
   - Fix segfault issues in runtime cleanup
   - Re-enable LLVM JIT with proper isolation

3. **Process Isolation**
   - Run each QIR test in a separate subprocess
   - No shared memory between tests
   - Natural cleanup on process exit

4. **Thread-Safe Runtime**
   - Make all runtime components truly thread-safe
   - Remove global registries
   - Use dependency injection for callbacks

## Impact

- **Performance**: Tests run slower due to serialization
- **Reliability**: Occasional failures if lock gets stuck
- **Maintenance**: Custom lock code adds complexity
- **Developer Experience**: Confusing failures when lock issues occur

## Action Items

- [ ] Create issue to track QIR runtime refactoring
- [ ] Prototype context-based runtime without globals
- [ ] Test LLVM cleanup fixes in isolated environment
- [ ] Consider subprocess-based test runner for QIR tests
- [ ] Remove file-based lock once issues are resolved

## References

- Original segfault issues: `QIR_SEGFAULT_ISSUE.md`
- LLVM JIT disabled: See `Cargo.toml` comments
- Runtime implementation: `crates/pecos-qir/src/runtime/`