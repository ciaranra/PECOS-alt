# pecos-clib-pcg

Rust wrapper for the PCG C library used in PECOS.

This crate provides safe Rust bindings to the PCG random number generator implementation in C.

## Features

- `pcg32_random()` - Generate a 32-bit random number
- `pcg32_boundedrand(bound)` - Generate a random number in range [0, bound)
- `pcg32_frandom()` - Generate a random floating-point number in [0, 1)
- `pcg32_srandom(seed)` - Set the random seed

## Usage

This crate is primarily used internally by PECOS and exposed through the main `pecos` crate's prelude.

```rust
use pecos_clib_pcg::{random, boundedrand, frandom, srandom};

// Set seed for reproducibility
srandom(12345);

// Generate random values
let r1 = random();           // 32-bit random number
let r2 = boundedrand(100);   // Random number in [0, 100)
let r3 = frandom();          // Random float in [0, 1)
```

## Implementation

This crate uses the `cc` build dependency to compile the C implementation of PCG32 (64-bit state, 32-bit output).
When building from the PECOS workspace, it uses the local C source files. When used as a dependency from crates.io,
it automatically downloads the C source files from the PECOS GitHub repository.

The PCG implementation is based on the work by Melissa E. O'Neill. For more information, visit http://www.pcg-random.org
