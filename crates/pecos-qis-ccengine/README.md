# pecos-qis-ccengine

QIS Classical Control Engine for PECOS quantum simulation framework.

## Features

- **QisControlEngine**: Orchestrates between QisInterface and QisRuntime
- **Native Runtime**: Pure Rust implementation
- **Selene Runtime**: Integration with Selene runtime plugins
- **Automatic Runtime Discovery**: Finds Selene runtimes in adjacent repository

## Selene Runtime Support

### Available Runtimes

- `selene_simple_runtime()` - Basic Selene runtime
- `selene_soft_rz_runtime()` - Runtime with soft RZ gate support

### Automatic Building

If the Selene repository is found at `../selene` (relative to PECOS), the runtime plugins are automatically built when you build this crate:

```bash
# Just build normally - Selene runtimes are built automatically if found
cargo build --package pecos-qis-ccengine
```

The build script:
1. Detects the Selene repository at `../selene`
2. Builds `selene_simple_runtime` and `selene_soft_rz_runtime` if not already built
3. Places the .so files in `../selene/target/release/`
4. Skips building if .so files already exist (fast rebuilds)

This happens automatically - no configuration needed!

### Runtime Discovery

The runtime discovery searches in this order:
1. `PECOS_SELENE_DIR` environment variable
2. Adjacent Selene repository (`../selene/target/release/`)
3. Current project target directory
4. System library paths (`/usr/local/lib`, `/usr/lib`)
5. Cache directory (`~/.cache/pecos-selene-runtimes/`)

### Usage Example

```rust
use pecos_qis_ccengine::{qis_control_engine, selene_simple_runtime};
use pecos_engines::ClassicalControlEngineBuilder;

// Try to use Selene runtime, fallback to native
let engine = match selene_simple_runtime() {
    Ok(runtime) => qis_control_engine().runtime(runtime).build()?,
    Err(_) => qis_control_engine().build()?,  // Use native runtime
};
```

## Building from Source

### Prerequisites

- Rust toolchain
- (Optional) Selene repository cloned at `../selene` for runtime support

### Build Commands

```bash
# Build (automatically builds Selene runtimes if found)
cargo build --package pecos-qis-ccengine

# Run tests
cargo test --package pecos-qis-ccengine

# Run examples
cargo run --package pecos-qis-ccengine --example selene_runtimes_usage
```

## Dependencies

- `pecos-qis-interface`: QIS operation interface
- `pecos-qis-runtime-trait`: Runtime trait definitions
- `libloading`: Dynamic library loading for Selene plugins
- `dirs`: Cache directory management