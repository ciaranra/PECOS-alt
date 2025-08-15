#!/usr/bin/env python3
"""Test HUGR compilation with return values instead of arrays."""

# Let's manually write a simple Guppy program that should work
guppy_code = """from __future__ import annotations

from guppylang.decorator import guppy
from guppylang.std import quantum

@guppy
def main() -> bool:
    q = quantum.qubit()
    quantum.h(q)
    return quantum.measure(q)
"""

print("=== Manual Guppy Code ===")
print(guppy_code)

# Try to compile this directly
print("\n=== Testing HUGR compilation ===")

import tempfile
import importlib.util
from guppylang import guppy

# Write to temp file
with tempfile.NamedTemporaryFile(mode='w', suffix='.py', delete=False) as f:
    temp_file = f.name
    f.write(guppy_code)

try:
    # Import the module
    spec = importlib.util.spec_from_file_location("test_module", temp_file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    
    # Compile to HUGR
    print("Compiling to HUGR...")
    hugr_module = guppy.compile(module.main)
    print("HUGR compilation successful!")
    print(f"HUGR type: {type(hugr_module)}")
    
except Exception as e:
    print(f"HUGR compilation failed: {e}")
    import traceback
    traceback.print_exc()

finally:
    import os
    os.unlink(temp_file)