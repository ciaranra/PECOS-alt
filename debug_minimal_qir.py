#!/usr/bin/env python3
"""Minimal test to debug QIR segfault"""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import h, measure, qubit
from pecos.frontends.guppy_frontend import GuppyFrontend

@guppy
def simple_test() -> bool:
    q = qubit()
    return measure(q)

print("=== Creating frontend ===")
frontend = GuppyFrontend(use_rust_backend=True, llvm_convention="hugr")

print("=== Compiling function ===")
qir_file = frontend.compile_function(simple_test)
print(f"QIR file: {qir_file}")

print("=== Reading QIR content ===")
with open(qir_file, 'r') as f:
    content = f.read()
print(f"QIR content length: {len(content)} characters")
print("First 500 characters:")
print(content[:500])

print("\n=== Checking QIR format ===")
# Let's check if the QIR is malformed
from pecos_rslib import validate_qir_format_detailed

validation = validate_qir_format_detailed(str(qir_file))
print(f"QIR validation: {validation}")

print("\n=== Testing execute_qir directly ===")
from pecos_rslib import execute_qir

try:
    result = execute_qir(str(qir_file), 1, 42, None, None, llvm_convention="hugr")
    print(f"Success: {result}")
except Exception as e:
    print(f"Error: {e}")
    import traceback
    traceback.print_exc()

print("=== Script completed ===")