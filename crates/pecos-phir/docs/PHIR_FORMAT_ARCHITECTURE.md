# PHIR Format Architecture

## Three-Layer Serialization Strategy

PMIR uses a three-layer approach for serialization, each optimized for different use cases:

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   PHIR-JSON     │ ←→  │   PHIR-RON      │ ←→  │   PMIR          │
│ (User-friendly) │     │ (Debug/Bridge)  │     │  (In-memory)    │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        ↑                       ↑                        ↑
   External API          Debug/Bridge              Internal IR
   Stable, versioned     IR-mirroring              Can change freely
   Human readable        Compact                   Optimized for speed
```

## PHIR-JSON (External Interface)

**Purpose**: Stable, versioned, human-readable format for external users

**Example**:
```json
{
  "format": "PHIR/JSON",
  "version": "0.2.0",
  "module": {
    "name": "bell_circuit",
    "body": {
      "kind": "SSACFG",
      "blocks": [{
        "ops": [
          {"qop": "H", "args": ["%0"], "returns": ["%1"]},
          {"qop": "CNOT", "args": ["%1", "%2"], "returns": ["%3", "%4"]}
        ]
      }]
    }
  }
}
```

**Characteristics**:
- Verbose but clear
- Stable across PMIR changes
- Easy to generate from other tools
- Self-documenting

## PHIR-RON (Debug/Bridge Format)

**Purpose**: Serialization that closely mirrors the internal IR structure, useful for debugging and as a bridge between PHIR-JSON and PHIR

**Example**:
```ron
PhirRon(
    format: "PHIR/RON",
    version: "0.2.0",
    module: ModuleOp(
        name: "bell_circuit",
        body: Region(
            blocks: [
                Block(
                    operations: [
                        Instruction(
                            operation: Quantum(H),
                            operands: [SSAValue(0)],
                            results: [SSAValue(1)],
                        ),
                    ],
                ),
            ],
            kind: SSACFG,
        ),
    ),
)
```

**Characteristics**:
- Direct enum representation
- No string parsing for variants
- Compact but readable
- Natural Rust types

## PMIR (In-Memory)

**Purpose**: Efficient in-memory representation for compilation

**Characteristics**:
- Direct Rust structs/enums
- Optimized for traversal and mutation
- No serialization overhead
- Can change freely

## Benefits of Three-Layer Approach

### 1. Separation of Concerns
- **PHIR-JSON**: What users see and write
- **PHIR-RON**: Efficient serialization for tools
- **PMIR**: What the compiler works with

### 2. Progressive Lowering Visibility
```rust
// Users can request different serialization levels
module.to_phir_json(Level::High);     // Abstract operations
module.to_phir_ron(Level::Mid);        // Expanded protocols  
module.to_phir_json(Level::Low);       // Physical gates
```

### 3. Tool Integration
- JSON parsers exist in every language
- RON is perfect for Rust tooling
- PMIR stays efficient

### 4. Migration Path
```
Old PHIR v0.1 → Parse to PMIR → Serialize as PHIR-JSON v0.2
              ↘               ↗
                PHIR-RON v0.2
```

## Implementation Status

- [x] Basic PMIR structures
- [x] PHIR-JSON serialization framework
- [x] PHIR-RON serialization framework
- [ ] Full type serialization support
- [ ] Bidirectional conversion
- [ ] Version compatibility layer
- [ ] Pretty printing options

## Use Cases

### 1. Human Authoring (PHIR-JSON)
```json
{
  "qop": "PrepareLogicalPlus",
  "attributes": {
    "qec.code": "steane",
    "comment": "Initialize logical qubit"
  }
}
```

### 2. Tool Exchange (PHIR-RON)
```ron
Operation::Custom(CustomOp {
    dialect: "qec",
    name: "PrepareLogicalPlus",
    attributes: {
        "qec.code": String("steane"),
    },
})
```

### 3. Compiler Internal (PMIR)
```rust
// Direct manipulation, no serialization
let op = Operation::Custom(qec_prepare_plus);
optimizer.run(&mut op);
```

## Future Considerations

1. **Binary Format**: For very large programs, add PHIR-MSGPACK
2. **Streaming**: Support incremental parsing/serialization
3. **Compression**: Optional zstd compression for large files
4. **Schema**: Generate JSON Schema from Rust types