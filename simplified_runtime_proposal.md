# Simplified QIR Runtime Proposal

## Current Issues

The QIR runtime has become overly complex due to trying to solve LLVM-IR convention mismatches with runtime isolation:

1. **Dual Context System**: Thread-local contexts + global fallback
2. **Aggressive Cleanup**: Multiple `__quantum__rt__initialize()` calls + sleep
3. **Complex Synchronization**: Multiple mutexes and atomic counters
4. **Performance Impact**: 3+ second overhead per execution

## Root Cause Analysis

The complexity was added to solve "crashes" that were actually:
- PMIR backend generating wrong LLVM-IR convention (pointer vs integer)
- QIR format validation failing on valid HUGR code
- Entry point signature mismatches

**These are now fixed**, so we can simplify.

## Proposed Simplification

### 1. Remove Complex Context System
- Remove `runtime_context.rs` entirely
- Remove `ContextGuard` and thread-local storage
- Use simple global state with basic synchronization

### 2. Simplify Runtime Initialization
- Remove "aggressive cleanup" hack (lines 171-179 in qir_bindings.rs)
- Single `reset_qir_runtime()` call instead of multiple
- Remove arbitrary thread sleep

### 3. Streamline State Management
- Keep simple global counters for qubit/result allocation
- Use single global message builder
- Remove complex fallback mechanisms

### 4. Expected Performance Improvement
- Should reduce execution time from 3+ seconds to milliseconds
- Eliminate hanging in Python tests
- Allow higher shot counts in tests

## Implementation Plan

1. **Phase 1**: Create simplified runtime without context system
2. **Phase 2**: Update Python bindings to remove aggressive cleanup
3. **Phase 3**: Test performance improvement
4. **Phase 4**: Remove old complex system if tests pass

## Risk Assessment

- **Low Risk**: The LLVM-IR convention issues are now fixed
- **High Reward**: Major performance improvement expected
- **Rollback Plan**: Keep current code until simplified version is tested

Would you like me to implement this simplification?