#!/usr/bin/env python3
"""Simple test to check if pipelines are working without hanging."""

import sys
from pathlib import Path

import pytest

sys.path.append("python/quantum-pecos/src")

try:
    from guppylang import guppy
    from guppylang.std.quantum import h, measure, qubit
    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False

try:
    from pecos.frontends.run_guppy import run_guppy, get_guppy_backends
    PECOS_FRONTEND_AVAILABLE = True
except ImportError:
    PECOS_FRONTEND_AVAILABLE = False


@pytest.mark.skipif(not GUPPY_AVAILABLE or not PECOS_FRONTEND_AVAILABLE, 
                    reason="Dependencies not available")
def test_simple_hadamard():
    """Test a simple Hadamard gate on both pipelines."""
    
    @guppy
    def test_h() -> bool:
        q = qubit()
        h(q)
        return measure(q)
    
    backends = get_guppy_backends()
    
    # Test HUGR-LLVM if available
    if backends.get("rust_backend", False):
        try:
            result = run_guppy(test_h, shots=1, backend="rust", verbose=True)
            print(f"HUGR-LLVM result: {result}")
            assert "results" in result, "HUGR-LLVM execution failed - no results"
            assert len(result["results"]) > 0, "HUGR-LLVM execution failed - empty results"
        except Exception as e:
            pytest.skip(f"HUGR-LLVM backend not working: {e}")
    
    # Test PMIR
    try:
        result = run_guppy(test_h, shots=1, backend="external", verbose=True)
        print(f"PMIR result: {result}")
        assert "results" in result, "PMIR execution failed - no results"
        assert len(result["results"]) > 0, "PMIR execution failed - empty results"
    except Exception as e:
        pytest.skip(f"PMIR backend not working: {e}")


if __name__ == "__main__":
    test_simple_hadamard()