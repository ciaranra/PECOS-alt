#!/bin/bash
# Build the FFI to ByteMessage bridge as a shared library

echo "Building Selene FFI to ByteMessage bridge..."

# Compile the FFI bridge as a shared library
rustc --crate-type cdylib \
  --edition 2021 \
  -C opt-level=0 \
  -L target/debug/deps \
  --extern pecos_engines=target/debug/deps/libpecos_engines.rlib \
  --extern once_cell=target/debug/deps/libonce_cell-*.rlib \
  -o target/debug/libselene_ffi_bridge.so \
  crates/pecos-selene/src/selene_ffi_to_bytemessage.rs

echo "Built: target/debug/libselene_ffi_bridge.so"

# Check exported symbols
echo "Exported symbols:"
nm -D target/debug/libselene_ffi_bridge.so | grep selene_ | head -10