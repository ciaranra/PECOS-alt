#!/usr/bin/env python3
"""Simple test to verify our transformation is being called."""

import os

from guppylang import guppy
from pecos.compilation_pipeline import compile_guppy_to_hugr, compile_hugr_to_llvm


# Test simple arithmetic
@guppy
def simple_add(x: int, y: int) -> int:
    """Add two integers."""
    return x + y


print("Compiling to HUGR...")
hugr = compile_guppy_to_hugr(simple_add)

print("HUGR compiled successfully")
print(f"HUGR size: {len(hugr)} bytes")

# Set up debug logging
os.environ["RUST_LOG"] = "debug"

print("Compiling HUGR to LLVM...")
try:
    llvm = compile_hugr_to_llvm(hugr)
    print("SUCCESS! LLVM compiled successfully")
except Exception as e:  # noqa: BLE001
    print(f"Failed: {e}")
