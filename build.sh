#!/bin/bash
set -e
cargo build
cd crates/plugin-python-service && maturin develop --uv
