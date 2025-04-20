#!/bin/bash
set -e

make clean build test

cargo run --bin pecos run examples/phir/bell.json -s 10 -w 2 -p 0.2
cargo run --bin pecos run examples/qir/bell.ll -s 10 -w 2 -p 0.2
cargo run --bin pecos run examples/phir/bell.json -s 10 -w 1
cargo run --bin pecos run examples/qir/bell.ll -s 10 -w 1
cargo run --bin pecos run examples/phir/bell.json -s 10 -w 10
cargo run --bin pecos run examples/qir/bell.ll -s 10 -w 10
cargo run --example replaying_rng --package pecos-core
cargo run --example bell_state_replay --package pecos-qsim
