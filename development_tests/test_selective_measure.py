#!/usr/bin/env python3
"""Test selective measurement pattern - measuring only some qubits."""

# Test if we can selectively measure some qubits while keeping others
guppy_code = """from __future__ import annotations

from guppylang.decorator import guppy
from guppylang.std import quantum
from guppylang.std.builtins import array, owned, result

@guppy
def main() -> None:
    # Create quantum array
    q0 = quantum.qubit()
    q1 = quantum.qubit()
    q2 = quantum.qubit()
    qubits = array(q0, q1, q2)
    
    # Apply some gates
    quantum.h(qubits[0])
    quantum.cx(qubits[0], qubits[1])
    quantum.cx(qubits[1], qubits[2])
    
    # Unpack the array
    q0_out, q1_out, q2_out = qubits
    
    # Measure only the first two qubits
    c0 = quantum.measure(q0_out)
    c1 = quantum.measure(q1_out)
    
    # Keep q2_out for further operations
    quantum.h(q2_out)
    c2 = quantum.measure(q2_out)  # Measure later
    
    # Create result array
    c_out = array(c0, c1, c2)
    
    result("c", c_out)
"""

print("=== Testing selective measurement with unpacking ===")
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
    print("\nCompiling to HUGR...")
    hugr_module = guppy.compile(module.main)
    print("HUGR compilation successful!")
    print("\n✅ Selective measurement pattern WORKS!")
    print("You can measure some qubits while keeping others for later!")
    
except Exception as e:
    print(f"HUGR compilation failed: {e}")

finally:
    import os
    os.unlink(temp_file)