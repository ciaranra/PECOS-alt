# HUGR Measurement Architecture Issue

## Problem Summary

There is a fundamental architectural mismatch between HUGR's measurement model and PECOS's LlvmEngine execution model.

### HUGR Model (Immediate Measurements)
HUGR generates QIR code that:
1. Performs a measurement: `%result = call i32 @__quantum__qis__m__body(...)`
2. Immediately uses the result: `%is_one = icmp ne i32 %result, 0`
3. Returns the boolean result to the caller

### PECOS LlvmEngine Model (Deferred Measurements)
The LlvmEngine executes in phases:
1. **Command Generation Phase**: Run the QIR program to generate quantum commands
2. **Quantum Simulation Phase**: Execute the quantum circuit and get measurement results
3. **Result Collection Phase**: Collect and return the measurement results

## The Issue

HUGR-generated code tries to use measurement results during the command generation phase, but those results aren't available until after quantum simulation. This causes all measurements to return 0.

## Example

HUGR generates:
```llvm
%measurement_result = call i32 @__quantum__qis__m__body(i64 %qubit, i64 25)
%is_one = icmp ne i32 %measurement_result, 0  ; Uses result immediately!
store i1 %is_one, i1* %result_location
```

But in PECOS's deferred model, `@__quantum__qis__m__body` just records that a measurement should happen - it doesn't return the actual result.

## Attempted Solution

We tried to convert immediate measurements to deferred ones:
```llvm
; Convert to:
call void @__hugr__quantum__qis__m__body(i64 %qubit, i64 25)  ; Record measurement
%measurement_result = call i32 @__quantum__rt__result_get_one(i64 25)  ; Get result later
```

But this doesn't work because `@__quantum__rt__result_get_one` is still called during command generation, before the quantum simulation has run.

## Root Cause

The issue is that HUGR assumes a model where:
- Quantum operations and measurements happen immediately
- Classical computation can depend on measurement results
- The entire program runs sequentially

But PECOS LlvmEngine assumes:
- Quantum operations are recorded as commands first
- All quantum simulation happens in a separate phase
- Classical computation that depends on measurements must be deferred

## Workarounds

1. **Use MonteCarloEngine**: The current implementation uses `MonteCarloEngine::run_with_noise_model()` which properly handles the quantum simulation, but measurement results are still 0 during QIR execution.

2. **Post-process Results**: Accept that the QIR program will see 0 for all measurements, and handle the actual measurement results through the Shot data structure returned by MonteCarloEngine.

3. **Modify HUGR Compiler**: The proper fix would be to modify the HUGR compiler to generate QIR that doesn't use measurement results during execution, but instead just records what measurements should be made.

## Impact

This issue means that HUGR-generated quantum programs that have classical logic dependent on measurement results (like conditional operations or returning measurement results) will not work correctly with PECOS.

Programs that only perform quantum operations and measurements without classical dependencies will work fine, as the measurement operations are recorded correctly.