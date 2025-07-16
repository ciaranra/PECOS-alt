# PECOS Decoders

This document describes the quantum error correction decoder implementations available in PECOS and how to use them.

## Overview

PECOS includes LDPC (Low-Density Parity-Check) decoders as optional components. These decoders are implemented as a separate crate to keep the core PECOS functionality lightweight while providing access to powerful decoding capabilities when needed.

The LDPC decoders come from external open-source projects developed by the quantum error correction community. PECOS provides Rust bindings and a unified interface to these excellent implementations. We are grateful to all the researchers and developers who have made their work openly available.

## Available Decoders

### LDPC Decoders (`pecos-ldpc-decoders`)
- **Description**: Low-Density Parity-Check decoders for quantum error correction
- **Algorithms**:
  - BP-OSD (Belief Propagation with Ordered Statistics Decoding)
  - BP-LSD (Belief Propagation with Localized Statistics Decoding)
  - MBP (Min-sum Belief Propagation)
  - Belief Find decoder
  - Flip decoder
  - Union Find decoder
  - SoftInfoBP decoder
- **Use cases**: Quantum LDPC codes, surface codes, hypergraph product codes, CSS codes
- **Language**: C++ with Rust bindings

## Architecture

The decoder subsystem follows a modular architecture with clear separation of concerns:

### Core Components

- **`pecos-decoder-core`**: The foundation crate containing:
  - Common traits that all decoders must implement (`Decoder`, `CssDecoder`, `BatchDecoder`, `SoftDecoder`)
  - Standard error types and result types
  - Shared utilities for decoder implementations

- **`pecos-ldpc-decoders`**: The LDPC decoder implementation crate
  - C++ implementation with Rust FFI bindings
  - Multiple decoder variants and algorithms
  - Comprehensive test coverage

- **`pecos-decoders`**: Meta-crate that provides a unified interface
  - Re-exports all decoder functionality
  - Feature flags for selective inclusion
  - Simplified API for common use cases

- **`pecos-build-utils`**: Build utilities for C++ compilation
  - Common build scripts and configuration
  - C++ compiler detection and setup
  - Caching mechanisms for faster builds

## Installation and Usage

### Basic Installation

To use LDPC decoders in your project, add this to your `Cargo.toml`:

```toml
[dependencies]
pecos-decoders = { version = "0.1.1", features = ["ldpc"] }
```

### Building from Source

```bash
# Build with LDPC decoders
cargo build --package pecos-decoders --features ldpc

# Build all decoders (currently just LDPC)
cargo build --package pecos-decoders --all-features

# Run tests
cargo test --package pecos-decoders --features ldpc
```

### Using the Makefile

The project includes convenient Makefile targets:

```bash
# Show available decoders
make decoder-info

# Build specific decoder
make build-decoder DECODER=ldpc

# Build all decoders
make build-decoders

# Test specific decoder
make test-decoder DECODER=ldpc

# Test all decoders
make test-decoders
```

## Usage Examples

### Basic LDPC Decoding

```rust
use pecos_decoders::{BpOsdDecoder, CssCode, DecodingResult};

// Create a CSS code from parity check matrices
let hx = vec![/* your Hx matrix */];
let hz = vec![/* your Hz matrix */];
let css_code = CssCode::new(hx, hz)?;

// Create decoder with default parameters
let mut decoder = BpOsdDecoder::new(css_code);

// Decode a syndrome
let syndrome = vec![/* your syndrome */];
let result: DecodingResult = decoder.decode(&syndrome)?;

println!("Correction: {:?}", result.correction);
println!("Converged: {}", result.converged);
```

### Advanced Configuration

```rust
use pecos_decoders::{BpOsdDecoder, BpMethod, BpSchedule, OsdMethod};

// Configure BP parameters
let bp_config = BpOsdDecoder::builder()
    .max_iterations(100)
    .bp_method(BpMethod::MinSum)
    .schedule(BpSchedule::Parallel)
    .osd_method(OsdMethod::Exhaustive)
    .osd_order(10)
    .build(css_code)?;
```

### Using Different LDPC Decoder Variants

```rust
use pecos_decoders::{
    BpLsdDecoder,
    BeliefFindDecoder,
    FlipDecoder,
    UnionFindDecoder,
};

// BP-LSD decoder
let mut bp_lsd = BpLsdDecoder::new(css_code.clone());
bp_lsd.set_bits_per_step(1);

// Belief Find decoder
let mut belief_find = BeliefFindDecoder::new(css_code.clone());
belief_find.set_uf_method(UfMethod::Inversion);

// Flip decoder
let mut flip = FlipDecoder::new(css_code.clone());
flip.set_max_iterations(100);

// Union Find decoder
let mut uf = UnionFindDecoder::new(css_code.clone());
```

## Build Configuration

### Environment Variables

- `PECOS_CACHE_DIR`: Directory for caching downloaded decoder sources (default: `~/.cache/pecos-decoders`)
- `CC`/`CXX`: C/C++ compilers to use
- `DECODER_DISABLE_NATIVE_ARCH`: Set to 1 to disable CPU-specific optimizations

### Build Features

The LDPC decoder crate supports various build configurations:

```bash
# Debug build (faster compilation)
cargo build --package pecos-ldpc-decoders

# Release build (optimized)
cargo build --release --package pecos-ldpc-decoders

# With specific features
cargo build --package pecos-decoders --features ldpc
```

## Decoder Details

### LDPC Decoder Algorithms

#### BP-OSD (Belief Propagation with Ordered Statistics Decoding)
- Combines belief propagation with post-processing
- Excellent performance for quantum LDPC codes
- Configurable OSD order and method

#### BP-LSD (Belief Propagation with Localized Statistics Decoding)
- Localized version of OSD
- Better scaling for large codes
- Configurable localization parameters

#### MBP (Min-sum Belief Propagation)
- Efficient min-sum variant of BP
- Lower complexity than sum-product
- Good performance/complexity tradeoff

#### Belief Find
- Combines belief propagation with union-find
- Adaptive algorithm selection
- Good for codes with specific structure

#### Flip Decoder
- Simple bit-flipping algorithm
- Very fast but lower performance
- Good for real-time applications

#### Union Find
- Graph-based decoder
- Efficient for certain code structures
- Multiple algorithmic variants

## Performance Considerations

1. **Parallelization**: Most decoders support parallel execution
2. **Memory Usage**: LDPC decoders use sparse matrix representations
3. **Compilation**: Release builds are significantly faster
4. **Caching**: Build artifacts are cached for faster rebuilds

## Contributing

To contribute to the decoder implementations:

1. Add new algorithms to the appropriate decoder crate
2. Implement the required traits from `pecos-decoder-core`
3. Add comprehensive tests
4. Update this documentation

## License

The LDPC decoder implementations are based on open-source projects with their respective licenses. See individual crate directories for specific license information.
