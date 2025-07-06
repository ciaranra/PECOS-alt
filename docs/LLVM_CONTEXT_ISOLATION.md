# QIR Context Isolation Design

## Problem Statement

The QIR runtime currently uses global state which prevents proper parallel execution and causes cleanup issues during Python shutdown (Fatal Python error: Aborted).

### Current Global State Issues

1. **Global Static Variables in runtime.rs**:
   - `NEXT_QUBIT_ID`: Global counter for qubit allocation
   - `NEXT_RESULT_ID`: Global counter for result allocation  
   - `MESSAGE_BUILDER`: Global message builder for quantum operations
   - `RUNTIME_STATE`: Global runtime state with measurement results
   - `LAST_SHOT`: Global storage for last shot results

2. **Problems with Global State**:
   - Prevents true parallel execution of QIR programs
   - Race conditions when multiple threads execute QIR
   - Cleanup ordering issues causing aborts during Python shutdown
   - No isolation between different QIR executions

## Implemented Solution: Execution Guard

As an immediate fix for the abort issue, we've implemented an execution guard that:

1. **Tracks active executions** to prevent cleanup during ongoing work
2. **Registers Python atexit handler** to coordinate shutdown
3. **Prevents the abort** by ensuring proper cleanup ordering

This is implemented in `qir_execution_guard.rs` and integrated into `qir_bindings.rs`.

## Future Solution: Context-Based Runtime

### Design Goals

1. **Complete isolation** - Each QIR execution gets its own context
2. **True parallelism** - Multiple QIR programs can execute simultaneously
3. **No global state** - All state contained within context objects
4. **Backward compatibility** - Existing QIR programs continue to work

### Proposed Architecture

```rust
// Thread-local context storage
thread_local! {
    static CURRENT_CONTEXT: RefCell<Option<Arc<Mutex<RuntimeContext>>>> = RefCell::new(None);
}

// Runtime context holding all state
struct RuntimeContext {
    message_builder: ByteMessageBuilder,
    state: RuntimeState,
    last_shot: Option<Shot>,
    next_qubit_id: usize,
    next_result_id: usize,
}

// RAII guard for setting context
struct ContextGuard { ... }

// Execute with isolated context
fn with_isolated_context<F, R>(f: F) -> R { ... }
```

### Implementation Steps

1. **Create RuntimeContext structure** (started in `runtime_context.rs`)
   - Move all global state into the context
   - Implement context lifecycle management

2. **Update QIR runtime functions**
   - Modify all `__quantum__rt__*` functions to use thread-local context
   - Maintain C ABI compatibility

3. **Update LlvmEngine**
   - Pass context through execution pipeline
   - Each engine instance gets its own context

4. **Gradual migration**
   - Keep backward compatibility layer
   - Deprecate global state over time

### Benefits

1. **True parallel execution** - Each thread/execution has isolated state
2. **No cleanup issues** - Context destroyed with execution
3. **Better testing** - Can test with different contexts
4. **Future extensibility** - Easy to add per-context configuration

### Challenges

1. **Static library integration** - Runtime is compiled as static library
2. **C ABI compatibility** - Must maintain existing function signatures
3. **Thread-local storage** - Need careful management across language boundaries

## Next Steps

1. Complete the context-based runtime implementation
2. Update LlvmEngine to use contexts
3. Add comprehensive tests for parallel execution
4. Benchmark performance with context isolation
5. Document migration path for users

## References

- LLVM best practices: One context per thread
- Similar approaches in other JIT compilers
- Thread-local storage in Rust FFI