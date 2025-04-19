#!/bin/bash
set -e

make clean build test

cargo run --bin pecos run examples/phir/bell.json -s 10 -w 2 -p 0.2
cargo run --bin pecos run examples/qir/bell.ll -s 10 -w 2 -p 0.2
