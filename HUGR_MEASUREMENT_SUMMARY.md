# HUGR Measurement Issue - Summary

## What We Found

The issue causing HUGR quantum circuits to return all zeros for measurements has been identified:

1. **Root Cause**: HUGR generates QIR code that expects immediate measurement results, but PECOS QirEngine uses a deferred measurement model where measurements are only resolved after quantum simulation.

2. **The Problem in Detail**:
   - HUGR generates: `%result = call i32 @__quantum__qis__m__body(...)`
   - Then immediately uses: `%is_one = icmp ne i32 %result, 0`
   - But in PECOS, the measurement function just records that a measurement should happen and returns 0
   - The actual measurement results aren't available until after quantum simulation completes

3. **Why This Happens**:
   - QirEngine runs in phases:
     1. Command Generation: Run QIR program to collect quantum operations
     2. Quantum Simulation: Execute the quantum circuit
     3. Result Collection: Get measurement results
   - HUGR code tries to use measurement results in phase 1, before they exist

## What We Fixed

1. **Added MonteCarloEngine Integration**: The `QirEngine::run()` method now uses `MonteCarloEngine` to properly execute quantum simulation

2. **Improved Result Extraction**: Enhanced the code to look for measurement results in various register names, not just "c"

3. **Added Debug Output**: Added extensive debugging to understand the flow

4. **Documented the Issue**: Created comprehensive documentation of the architectural mismatch

## What Still Doesn't Work

Despite the fixes, HUGR-generated quantum programs that return measurement results still return all zeros because:

1. The HUGR-generated QIR uses measurement results during execution
2. These results are always 0 during the command generation phase
3. The actual quantum measurement results are available in the Shot data, but the HUGR program has already returned 0

## The Fundamental Issue

This is an architectural mismatch between:
- **HUGR's Model**: Immediate measurements with sequential execution
- **PECOS's Model**: Deferred measurements with phased execution

## Possible Solutions

1. **Modify HUGR Compiler**: Change how HUGR generates QIR to not use measurement results during execution
2. **Create a HUGR-specific Engine**: Build a new engine that executes HUGR programs differently
3. **Post-process Results**: Accept that HUGR programs return 0 and extract real results from Shot data

## Current Status

- Quantum operations (H, CX, etc.) are correctly recorded and simulated
- Measurements are correctly performed by the quantum simulator
- The issue is only with getting measurement results back to the HUGR program
- Programs that don't return measurement results should work fine