#!/usr/bin/env python3
"""Debug QIR content generation"""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import h, measure, qubit
from pecos.frontends.guppy_frontend import GuppyFrontend

@guppy
def measure_only() -> bool:
    """Just measure a qubit - no gates"""
    q = qubit()
    return measure(q)

@guppy
def hadamard_measure() -> bool:
    """Hadamard then measure"""
    q = qubit()
    h(q)
    return measure(q)

print("Creating frontend...")
frontend = GuppyFrontend(use_rust_backend=True, llvm_convention="hugr")

print("\n=== Testing measure_only ===")
qir_file1 = frontend.compile_function(measure_only)
print(f"QIR file: {qir_file1}")

with open(qir_file1, 'r') as f:
    content1 = f.read()

print("QIR content for measure_only:")
print(content1)
print(f"Length: {len(content1)} chars")

# Check for quantum operations
if "__quantum__qis__h__body" in content1:
    print("ERROR: Found H gate in measure-only function!")
if "__quantum__qis__m__body" in content1 or "__quantum__qis__mz__body" in content1:
    print("Found measurement call")
else:
    print("WARNING: No measurement call found!")

print("\n=== Testing hadamard_measure ===")
qir_file2 = frontend.compile_function(hadamard_measure)
print(f"QIR file: {qir_file2}")

with open(qir_file2, 'r') as f:
    content2 = f.read()

print("QIR content for hadamard_measure:")
print(content2[:500] + "...")
print(f"Length: {len(content2)} chars")

# Check for quantum operations
if "__quantum__qis__h__body" in content2:
    print("Found H gate")
if "__quantum__qis__m__body" in content2 or "__quantum__qis__mz__body" in content2:
    print("Found measurement call")
else:
    print("WARNING: No measurement call found!")