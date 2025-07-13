# Known Issues with Guppy Integration in PECOS

This document describes known limitations and issues with the Guppy language integration in PECOS.

## 1. Qubit Allocation in Loops

**Issue**: Qubit allocation within loops (both `for` and `while` loops) causes a panic in the quantum simulator.

**Symptoms**: 
- Error: `index out of bounds: the len is 2 but the index is 2`
- Location: `pecos-qsim/src/state_vec.rs:632:35`

**Example Code That Fails**:
```python
@guppy
def parity_test() -> bool:
    parity = False
    for i in range(4):
        q = qubit()  # Allocation inside loop
        h(q)
        if measure(q):
            parity = not parity
    return parity
```

**Root Cause**: The Guppy compiler appears to optimize qubit allocation by hoisting it out of the loop body, causing all loop iterations to use the same qubit ID. However, `MeasureFree` operations consume the qubit, leading to conflicts.

**Workaround**: Unroll loops manually or restructure code to avoid qubit allocation within loop bodies.

## 2. Conditional Quantum Operations Based on Measurement Results

**Issue**: Quantum operations that depend on measurement results do not execute correctly.

**Symptoms**: 
- Conditional quantum gates (e.g., `if measure(q1): x(q2)`) always behave as if the condition is false
- The `__quantum__qis__m__body` function always returns 0 in PECOS's deferred measurement model

**Example Code That Fails**:
```python
@guppy
def conditional_test() -> tuple[bool, bool]:
    q1 = qubit()
    h(q1)
    m1 = measure(q1)
    
    q2 = qubit()
    if m1:  # This condition is never true during quantum execution
        x(q2)
    m2 = measure(q2)
    
    return m1, m2
```

**Root Cause**: Architectural mismatch between HUGR's immediate measurement model and PECOS's deferred measurement model. PECOS uses separate ClassicalControlEngine and QuantumEngine components that communicate through an EngineSystem, but the current LLVM generation doesn't properly handle this interaction.

**Status**: This requires significant architectural changes to properly implement the back-and-forth communication between classical and quantum engines.

## 3. Empty Loops

**Issue**: Loops with `range(0)` cause a different panic.

**Symptoms**:
- Error: `LLVM: Runtime returned no shot results after finalization`
- Location: `pecos-llvm-runtime/src/engine.rs:312:21`

**Root Cause**: The LLVM runtime expects at least one measurement result, but empty loops produce no measurements.

## Mitigation Strategies

1. **For Loop Issues**: Use explicit qubit allocation outside loops or unroll loops manually
2. **For Conditional Operations**: Structure programs to avoid quantum operations that depend on measurement results
3. **For Testing**: Mark affected tests with appropriate skip decorators until these issues are resolved

## Future Work

These issues require fixes at the Guppy compiler level or significant changes to the HUGR-to-LLVM compilation strategy to properly handle:
- Dynamic qubit allocation and deallocation within loop bodies
- Proper interaction between classical control flow and quantum operations in the deferred measurement model