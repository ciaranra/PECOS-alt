#!/usr/bin/env python3
"""Test using Option[qubit] arrays for selective measurement."""

# Test pattern using Option[qubit] to allow selective consumption
guppy_code = """from __future__ import annotations

from guppylang.decorator import guppy
from guppylang.std import quantum
from guppylang.std.builtins import array, owned, result
from guppylang.std.option import Option, some, nothing

@guppy
def selective_measure(qubits: array[Option[quantum.qubit], 3], index: int, bits: array[bool, 3]) -> None:
    # Try to measure one specific qubit and replace with nothing
    if qubits[index].is_some():
        q = qubits[index].unwrap()
        bits[index] = quantum.measure(q)
        # Replace with nothing
        qubits[index] = nothing()

@guppy  
def main() -> None:
    # Create quantum array wrapped in Options
    q0 = some(quantum.qubit())
    q1 = some(quantum.qubit())  
    q2 = some(quantum.qubit())
    qubits = array(q0, q1, q2)
    
    # Classical array
    c = array(False, False, False)
    
    # Apply gates - need to unwrap first
    if qubits[0].is_some():
        quantum.h(qubits[0].unwrap())  # This might not work...
    
    # Try selective measurement
    selective_measure(qubits, 0, c)
    
    result("c", c)
"""

print("=== Testing Option[qubit] array pattern ===")
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
    print("Option[qubit] pattern WORKS!")
    
except Exception as e:
    print(f"HUGR compilation failed: {e}")
    print("\nThe issue might be that unwrap() consumes but we can't operate on consumed values...")

finally:
    import os
    os.unlink(temp_file)