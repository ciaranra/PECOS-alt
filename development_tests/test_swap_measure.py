#!/usr/bin/env python3
"""Test swapping pattern for selective measurement."""

# Test if we can swap qubits to measure them individually
guppy_code = """from __future__ import annotations

from guppylang.decorator import guppy
from guppylang.std import quantum
from guppylang.std.builtins import array, owned, result
from guppylang.std.mem import mem_swap

@guppy
def main() -> None:
    # Create quantum array
    q0 = quantum.qubit()
    q1 = quantum.qubit()
    q2 = quantum.qubit()
    qubits = array(q0, q1, q2)
    
    # Classical array
    c = array(False, False, False)
    
    # Apply some gates (this works with subscripting)
    quantum.h(qubits[0])
    quantum.cx(qubits[0], qubits[1])
    quantum.cx(qubits[1], qubits[2])
    
    # Try to extract and measure a qubit
    # First, create a temporary qubit to swap with
    temp = quantum.qubit()
    
    # Try swapping qubits[0] with temp - will this work?
    # mem_swap(qubits[0], temp)  # Probably won't work due to subscripting
    
    # Alternative: unpack the entire array, measure, and repack
    q0_out, q1_out, q2_out = qubits  # Unpack the array
    c0 = quantum.measure(q0_out)  # Measure individual qubits
    c1 = quantum.measure(q1_out)
    c2 = quantum.measure(q2_out)
    
    # Create new array with results
    c_out = array(c0, c1, c2)
    
    result("c", c_out)
"""

print("=== Testing array unpacking for measurement ===")
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
    print("Array unpacking pattern WORKS!")
    
except Exception as e:
    print(f"HUGR compilation failed: {e}")

finally:
    import os
    os.unlink(temp_file)