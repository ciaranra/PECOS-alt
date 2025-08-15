#!/usr/bin/env python3
"""Test SLR-like patterns with compile-time known sizes."""

from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure

# Typical SLR pattern - QReg with known size, selective measurements
prog = Main(
    q := QReg("q", 7),  # Steane code size - known at compile time
    c := CReg("c", 7),
    
    # Apply gates (subscripting works fine)
    qubit.H(q[0]),
    qubit.H(q[4]),
    qubit.H(q[6]),
    
    qubit.CX(q[0], q[1]),
    qubit.CX(q[4], q[5]),
    qubit.CX(q[6], q[3]),
    
    # Measure some qubits at different times (typical in QEC)
    Measure(q[0]) > c[0],
    Measure(q[1]) > c[1],
    # Do more operations
    qubit.CX(q[2], q[3]),
    # Measure more
    Measure(q[2]) > c[2],
    Measure(q[3]) > c[3],
    Measure(q[4]) > c[4],
    Measure(q[5]) > c[5],
    Measure(q[6]) > c[6],
)

print("=== Current SLR → Guppy (broken for HUGR) ===")
current_code = SlrConverter(prog).guppy()
print(current_code)

print("\n=== What it should generate for HUGR ===")
fixed_code = """from __future__ import annotations

from guppylang.decorator import guppy
from guppylang.std import quantum
from guppylang.std.builtins import array, owned, result



@guppy
def main() -> None:
    q = array(quantum.qubit() for _ in range(7))
    
    # Apply gates (subscripting works fine)
    quantum.h(q[0])
    quantum.h(q[4])
    quantum.h(q[6])
    quantum.cx(q[0], q[1])
    quantum.cx(q[4], q[5])
    quantum.cx(q[6], q[3])
    
    # Unpack for measurements (since size is known: 7)
    q0, q1, q2, q3, q4, q5, q6 = q
    
    # Measure some qubits
    c0 = quantum.measure(q0)
    c1 = quantum.measure(q1)
    
    # Continue with remaining qubits
    quantum.cx(q2, q3)
    
    # Measure the rest
    c2 = quantum.measure(q2)
    c3 = quantum.measure(q3)
    c4 = quantum.measure(q4)
    c5 = quantum.measure(q5)
    c6 = quantum.measure(q6)
    
    # Pack results
    c = array(c0, c1, c2, c3, c4, c5, c6)
    result("c", c)
"""

print(fixed_code)

print("\n=== Testing fixed code ===")
import tempfile
import importlib.util
from guppylang import guppy

# Write to temp file
with tempfile.NamedTemporaryFile(mode='w', suffix='.py', delete=False) as f:
    temp_file = f.name
    f.write(fixed_code)

try:
    # Import the module
    spec = importlib.util.spec_from_file_location("test_module", temp_file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    
    # Compile to HUGR
    print("Compiling to HUGR...")
    hugr_module = guppy.compile(module.main)
    print("✅ HUGR compilation successful!")
    print("\nThis pattern works for SLR with known QReg sizes!")
    
except Exception as e:
    print(f"❌ HUGR compilation failed: {e}")

finally:
    import os
    os.unlink(temp_file)