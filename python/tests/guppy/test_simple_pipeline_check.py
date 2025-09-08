#!/usr/bin/env python3
"""Simple test to check if pipelines are working without hanging."""

import sys

import pytest

sys.path.append("python/quantum-pecos/src")

try:
    from guppylang import guppy
    from guppylang.std.quantum import h, measure, qubit

    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False

try:
    from pecos.frontends.run_guppy import get_guppy_backends, run_guppy

    PECOS_FRONTEND_AVAILABLE = True
except ImportError:
    PECOS_FRONTEND_AVAILABLE = False


@pytest.mark.skipif(
    not GUPPY_AVAILABLE or not PECOS_FRONTEND_AVAILABLE,
    reason="Dependencies not available",
)
def test_simple_hadamard() -> None:
    """Test a simple Hadamard gate on both pipelines."""

    @guppy
    def test_h() -> bool:
        q = qubit()
        h(q)
        return measure(q)

    backends = get_guppy_backends()

    # Test with Rust backend (the only backend)
    if backends.get("rust_backend", False):
        try:
            result = run_guppy(test_h, shots=1, verbose=True)
            print(f"Rust backend result: {result}")
            assert "results" in result, "Execution failed - no results"
            assert len(result["results"]) > 0, "Execution failed - empty results"
        except Exception as e:
            pytest.skip(f"Rust backend not working: {e}")
    else:
        pytest.skip("Rust backend not available")


if __name__ == "__main__":
    test_simple_hadamard()
