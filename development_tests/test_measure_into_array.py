#!/usr/bin/env python3
"""Test measuring qubits into pre-existing classical array."""

# Let's manually write and test the pattern we want
guppy_code = """from __future__ import annotations

from guppylang.decorator import guppy
from guppylang.std import quantum
from guppylang.std.builtins import array, owned, result

N = guppy.nat_var("N")

@guppy
def measure_into_bits(qubits: array[quantum.qubit, N] @ owned, bits: array[bool, N]) -> None:
    for i in range(N):
        bits[i] = quantum.measure(qubits[i])

@guppy  
def main() -> None:
    # Create quantum and classical arrays
    q = array(quantum.qubit() for _ in range(3))
    c = array(False for _ in range(3))
    
    # Apply some gates
    quantum.h(q[0])
    quantum.cx(q[0], q[1])
    quantum.cx(q[1], q[2])
    
    # Try to measure into the classical array
    measure_into_bits(q, c)
    
    result("c", c)
"""

print("=== Testing measure into array pattern ===")
print(guppy_code)

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
    print("This pattern WORKS!")
    
except Exception as e:
    print(f"HUGR compilation failed: {e}")
    print("Let's try a simpler version...")

finally:
    import os
    os.unlink(temp_file)