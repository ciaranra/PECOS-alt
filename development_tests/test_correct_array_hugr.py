#!/usr/bin/env python3
"""Test HUGR compilation with correct array patterns from guppylang."""

from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure

# Simple program that creates qubits and measures them correctly
prog = Main(
    q := QReg("q", 2),
    c := CReg("c", 2),
    # Apply some gates
    qubit.H(q[0]),
    qubit.CX(q[0], q[1]),
    # Measure everything (using the array measurement pattern)
    Measure(q[0]) > c[0],
    Measure(q[1]) > c[1],
)

print("=== Current Generated Code ===")
guppy_code = SlrConverter(prog).guppy()
print(guppy_code)

print("=== What it should be for HUGR ===")
corrected_code = """from __future__ import annotations

from guppylang.decorator import guppy
from guppylang.std import quantum
from guppylang.std.builtins import array, owned, result



@guppy
def main() -> None:
    q = array(quantum.qubit() for _ in range(2))
    quantum.h(q[0])
    quantum.cx(q[0], q[1])
    # Use measure_array for proper linearity
    c = quantum.measure_array(q)
    result("c", c)
"""

print(corrected_code)

print("\n=== Testing corrected code ===")
import tempfile
import importlib.util
from guppylang import guppy

# Write to temp file
with tempfile.NamedTemporaryFile(mode='w', suffix='.py', delete=False) as f:
    temp_file = f.name
    f.write(corrected_code)

try:
    # Import the module
    spec = importlib.util.spec_from_file_location("test_module", temp_file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    
    # Compile to HUGR
    print("Compiling corrected code to HUGR...")
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