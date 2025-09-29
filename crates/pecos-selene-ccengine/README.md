# PECOS Selene Classical Control Engine

A proof-of-concept classical control engine that uses Selene runtime plugins for control flow while generating PECOS ByteMessages.

## Status: Proof of Concept

This is a POC demonstrating how we can:
1. Load Selene runtime plugins (.so files)
2. Use them with QIS programs
3. Generate ByteMessages for PECOS

## Architecture

```
QIS Program (LLVM IR)
    ↓ calls ___* functions
pecos-qis-runtime
    ↓ needs to forward to
selene_runtime_* functions
    ↓ handled by
Selene Runtime Plugin
    ↓ queues operations
get_next_operations()
    ↓ callbacks convert to
ByteMessages
```

## Key Components

- `runtime_plugin.rs`: FFI bindings to load Selene runtime plugins
- `bridge.rs`: Converts runtime operations to ByteMessages
- `engine.rs`: ClassicalControlEngine implementation
- `ffi_bridge.rs`: Provides selene_runtime_* functions

## Next Steps

1. Fix compilation issues (mainly around FFI and trait implementations)
2. Modify pecos-qis-runtime to optionally forward to selene_runtime_* functions
3. Test with actual Selene runtime plugins
4. Handle measurement feedback and control flow

## Challenges

The main challenge is bridging between:
- QIS programs that call `___*` functions
- Selene runtimes that expect `selene_runtime_*` functions
- PECOS that needs ByteMessages

This POC shows the architecture is feasible but needs refinement for production use.