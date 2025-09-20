# pecos-selene

Selene simulator integration for PECOS.

This crate provides support for compiling and executing programs using Selene within the PECOS simulation framework.

## Features

- Compile Selene programs to quantum circuits
- Integration with PECOS simulation engines
- Support for Selene's high-level constructs

## Usage

```rust
use pecos_selene::selene_engine;

// Create a Selene engine with a program
let engine = selene_engine()
    .source("quantum algorithm code")
    .build()?;
```

For more information about Selene, see the [Selene documentation](https://github.com/CQCL/selene).
