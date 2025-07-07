# PHIR-JSON as PMIR Serialization Format

## Overview

PHIR-JSON is evolving to become the stable, human-readable serialization format for PMIR. This creates a clean separation between:

- **PHIR-JSON**: The stable external API and serialization format
- **PMIR**: The flexible internal compiler IR

## Design Principles

### 1. Direct Mapping
PHIR-JSON directly represents PMIR concepts in JSON format:

```
PMIR (in-memory) ←→ PHIR-JSON (serialized)
```

### 2. Stability vs Flexibility
- **PHIR-JSON**: Versioned, stable, backward compatible
- **PMIR**: Free to evolve for better optimizations

### 3. Human Readability
While being a direct serialization, PHIR-JSON remains human-readable and writable.

## Example Mapping

### Simple Quantum Circuit

**PMIR (Rust code):**
```rust
let module = ModuleOp::new("bell_pair");
let func = FuncOp::new("main", function_type);
let block = Block::new(Some("entry"))
    .with_instruction(quantum_h(q0))
    .with_instruction(quantum_cx(q0, q1))
    .with_instruction(measure(q0));
```

**PHIR-JSON (v0.2 proposal):**
```json
{
  "format": "PHIR/JSON",
  "version": "0.2.0",
  "module": {
    "name": "bell_pair",
    "body": {
      "kind": "SSACFG",
      "blocks": [{
        "label": "entry",
        "ops": [
          {
            "function": {
              "name": "main",
              "inputs": [{"type": "qubit"}, {"type": "qubit"}],
              "outputs": [{"type": "bit"}, {"type": "bit"}],
              "body": [{
                "kind": "SSACFG",
                "blocks": [{
                  "ops": [
                    {"qop": "H", "args": ["%0"], "returns": ["%2"]},
                    {"qop": "CNOT", "args": ["%2", "%1"], "returns": ["%3", "%4"]},
                    {"qop": "Measure", "args": ["%3"], "returns": ["%5"]}
                  ]
                }]
              }]
            }
          }
        ]
      }]
    }
  }
}
```

## Key Concepts

### 1. SSA Values
PHIR-JSON uses `%n` notation for SSA values (like MLIR):
```json
{"qop": "H", "args": ["%0"], "returns": ["%1"]}
```

### 2. Hierarchical Structure
Matches PMIR's Operation → Region → Block → Operation hierarchy:
```json
{
  "module": {
    "body": {           // Region
      "blocks": [{      // Block
        "ops": [...]    // Operations
      }]
    }
  }
}
```

### 3. Attributes
Extensible metadata system:
```json
{
  "qop": "H",
  "args": ["%0"],
  "attributes": {
    "duration": [50, "ns"],
    "noise.model": "depolarizing",
    "qec.protected": true
  }
}
```

### 4. Region Kinds
Explicit execution semantics:
```json
{
  "kind": "SSACFG",     // Control flow graph
  // or
  "kind": "Graph",      // Concurrent/dataflow
  // or  
  "kind": "Parallel"    // Explicit parallelism
}
```

## Migration from PHIR v0.1

### Compatibility Mode
PHIR v0.2 can read v0.1 files:
```json
{
  "format": "PHIR/JSON",
  "version": "0.2.0",
  "compatibility": "0.1",
  "ops": [
    // v0.1 style flat operations still work
    {"qop": "H", "args": [["q", 0]]}
  ]
}
```

### Automatic Upgrade
The parser can automatically upgrade v0.1 → v0.2:
- Flat ops → Single-block module
- Array indices → SSA values
- Variable definitions → Block arguments

## Benefits

1. **Single Model**: Learn PMIR concepts, use them everywhere
2. **Tool Compatibility**: PMIR analysis tools work on PHIR files
3. **Round-trip**: Can serialize PMIR → PHIR → PMIR without loss
4. **Progressive**: Start simple, add complexity as needed

## Implementation Status

- [x] Basic serialization infrastructure
- [ ] Full operation coverage
- [ ] Deserialization (PHIR → PMIR)
- [ ] v0.1 compatibility mode
- [ ] Validation and error reporting
- [ ] Pretty printing options

## Future Considerations

### Machine Operations
PHIR's machine operations (Transport, Idle, etc.) become custom operations in PMIR:
```json
{
  "mop": "Transport",
  "args": ["%q0"],
  "duration": [1.0, "ms"],
  "attributes": {
    "from_zone": "storage",
    "to_zone": "compute"
  }
}
```

### Streaming Support
For large programs, support streaming serialization:
```json
{"format": "PHIR/JSON", "version": "0.2.0", "streaming": true}
{"function": {"name": "func1", ...}}
{"function": {"name": "func2", ...}}
```

### Binary Format
For performance, could add a binary serialization format alongside JSON:
- PHIR-JSON: Human-readable
- PHIR-Binary: Fast serialization/deserialization

## Conclusion

By making PHIR-JSON a direct serialization of PMIR, we get:
- Cleaner architecture
- Better compatibility
- Easier maintenance
- More powerful capabilities

Users get a stable, versioned format while developers can evolve PMIR internally.