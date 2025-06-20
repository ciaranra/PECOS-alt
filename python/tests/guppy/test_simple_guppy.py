#!/usr/bin/env python3
"""Test simple Guppy compilation."""

import pytest
from guppylang import guppy
from guppylang.std.quantum import h, measure, qubit

pytestmark = pytest.mark.optional_dependency


@guppy
def random_bit() -> bool:
    """Generate a random bit using quantum superposition."""
    q = qubit()
    h(q)
    return measure(q)


def test_guppy_compilation_simple() -> None:
    """Test basic Guppy compilation."""
    print("Testing Guppy compilation...")

    # Compile the function
    compiled = guppy.compile_function(random_bit)
    print("Guppy function compiled successfully!")

    # Show the HUGR
    print(f"\nCompiled function: {compiled}")
    print(f"Package: {compiled.package}")

    # Try to get HUGR bytes
    hugr_bytes = compiled.package.to_bytes()
    print(f"\nHUGR bytes generated: {len(hugr_bytes)} bytes")
    assert len(hugr_bytes) > 0
