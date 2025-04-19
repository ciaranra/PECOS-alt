#!/bin/bash
set -e

make clean build test

(cd crates/plugin-python-service && maturin develop --uv)
cargo run --bin plugin-demo

cargo run --bin proto_bytemessage

cargo run --bin prototype

cargo run --bin pecos run examples/phir/bell.json -s 10 -w 2 -p 0.2
# TODO: Fix this:
cargo run --bin pecos run examples/qir/bell.ll -s 10 -w 2 -p 0.2

# TODO: Include running python/pecos-rslib/rust/tests/
