#!/usr/bin/env python3
"""Test quantum-only function that should work."""

from guppylang import guppy
from guppylang.std.quantum import h, measure, qubit
from pecos.compilation_pipeline import compile_guppy_to_hugr, compile_hugr_to_llvm


# Test quantum-only operations
@guppy
def quantum_only() -> bool:
    """Create and measure a quantum bit in superposition."""
    q = qubit()
    h(q)
    return measure(q)


print("Compiling quantum-only function to HUGR...")
hugr = compile_guppy_to_hugr(quantum_only)

print("Compiling HUGR to LLVM...")
try:
    llvm = compile_hugr_to_llvm(hugr)
    print("SUCCESS! Quantum-only function compiled successfully")
    print("LLVM IR length:", len(llvm))
except Exception as e:  # noqa: BLE001
    print(f"Failed: {e}")
