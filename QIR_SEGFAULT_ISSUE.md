# QIR Segfault Issue

## Problem
QIR execution causes segmentation faults when running with the PECOS CLI due to global state in the QIR runtime.

## Root Cause
The QIR runtime (`crates/pecos-qir/src/runtime.rs`) uses global static variables:
- `NEXT_QUBIT_ID`: AtomicUsize for qubit allocation
- `NEXT_RESULT_ID`: AtomicUsize for result allocation  
- `MESSAGE_BUILDER`: LazyLock<Mutex<ByteMessageBuilder>>
- `RUNTIME_STATE`: LazyLock<Mutex<RuntimeState>>
- `LAST_SHOT`: LazyLock<Mutex<Option<Shot>>>

These globals cause issues when:
1. Multiple workers try to execute QIR in parallel
2. The library is loaded/unloaded multiple times in the same process
3. Python loads many extension modules that conflict with LLVM symbols

## Attempted Solutions
1. **Arc<Mutex<>> wrapper for library sharing** - Still had global state conflicts
2. **Single-threaded execution** - Implemented via `requires_single_threaded_execution()` trait method
3. **Thread-local storage** - Started converting globals to thread_local! but this is a large refactoring

## Current Status
- Added `requires_single_threaded_execution()` to ClassicalEngine trait
- QirEngine returns `true` to force single-threaded execution
- MonteCarloEngine respects this and uses only 1 worker for QIR
- **Still segfaulting** - the issue is deeper than just parallel execution

## Proper Solution
The proper, idiomatic solution would be to:
1. Remove all global state from the QIR runtime
2. Make the runtime state instance-based rather than global
3. Pass the runtime state through the QIR library calls
4. Ensure each QirEngine instance has its own isolated runtime state

This would require significant refactoring of:
- `/home/ciaranra/Repos/cl_projects/gup/PECOS/crates/pecos-qir/src/runtime.rs`
- All the extern "C" functions that currently use global state
- The QirLibrary interface to pass runtime state

## Workaround
For now, QIR tests are failing with segfaults. The user wanted a proper solution rather than hacky workarounds, so the issue remains unresolved until the runtime can be properly refactored.