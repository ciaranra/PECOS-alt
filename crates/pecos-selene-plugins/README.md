# PECOS Selene Plugins

This crate provides runtime plugins for PECOS-Selene integration.

## Architecture

### Components

1. **ByteMessageSimulator** (`lib.rs`)
   - Implements `RuntimeInterface` for Selene
   - Collects quantum operations and converts them to ByteMessage format
   - Handles measurement results from PECOS quantum engines
   - Exported as a Selene runtime plugin via `export_runtime_plugin!`

2. **Plugin Builder** (`plugin_builder.rs`)
   - Builds Selene-compatible plugins from LLVM programs
   - Replicates Selene's Python build process in Rust
   - Supports LLVM IR and bitcode inputs
   - Generates shared libraries (.so) that can be loaded by Selene runtimes

3. **Execution Plugin** (`execution_plugin.rs`)
   - Provides runtime environment for Selene program plugins
   - Implements functions that program plugins expect:
     - `setup()` - Initialize execution environment
     - `teardown()` - Clean up after execution
     - `get_tc()` - Get time cursor
     - `get_next_operations()` - Get next batch of quantum operations
   - Bridges between Selene's program plugin interface and PECOS's execution model

4. **HUGR Compiler** (`hugr_compiler.rs`)
   - Utilities for compiling HUGR to LLVM
   - Integration point for HUGR-to-LLVM compilation pipeline

## Usage

### As a Selene Runtime Plugin

The ByteMessageSimulator can be used as a drop-in replacement for Selene's standard simulators:

```rust
use pecos_selene_plugins::ByteMessageSimulatorFactory;
use selene_core::runtime::RuntimeInterfaceFactory;

let factory = ByteMessageSimulatorFactory::default();
let runtime = factory.init(n_qubits, start_time, &[])?;
```

### Building Custom Plugins

Use the PluginBuilder to create Selene-compatible plugins from LLVM code:

```rust
use pecos_selene_plugins::plugin_builder::{PluginBuildConfig, LLVMSource};

let config = PluginBuildConfig {
    name: "my_plugin".to_string(),
    llvm_source: LLVMSource::IRString(llvm_ir),
    output_dir: output_path,
    verbose: false,
    link_flags: vec![],
    target_triple: None,
};

let plugin_path = build_plugin(&config)?;
```

## Integration with PECOS

This crate serves as the bridge between:
- PECOS's ByteMessage-based quantum communication protocol
- Selene's runtime plugin architecture
- LLVM-based quantum program compilation

The plugins built here can be loaded by:
- `SeleneEngine` for direct execution
- `SeleneSimpleRuntimeEngine` for simplified runtime execution
- Any Selene-compatible runtime system
