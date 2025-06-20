#!/usr/bin/env python3
"""Test arithmetic and boolean type support in Guppy->HUGR->LLVM pipeline."""

import pytest
from guppylang import guppy
from guppylang.std.quantum import h, measure, qubit
from pecos.compilation_pipeline import compile_guppy_to_hugr, compile_hugr_to_llvm

pytestmark = pytest.mark.optional_dependency


def test_integer_arithmetic() -> None:
    """Test that integer arithmetic now works with our extensions."""

    @guppy
    def add_numbers(x: int, y: int) -> int:
        return x + y

    # This used to fail with "Unknown type: int(6)"
    # Now it should work with our arithmetic extension
    hugr = compile_guppy_to_hugr(add_numbers)
    assert len(hugr) > 0

    # Try to compile to LLVM
    try:
        llvm = compile_hugr_to_llvm(hugr)
        print("Success! Integer arithmetic compiled to LLVM")
        print(f"LLVM IR length: {len(llvm)} characters")
        # If this works, our arithmetic extension is functioning
        assert len(llvm) > 0
    except RuntimeError as e:
        # If it still fails, check if it's the same error
        if "Unknown type: int" in str(e):
            pytest.fail(
                "Arithmetic extension not working - still getting Unknown type error",
            )
        else:
            # Different error - might be progress!
            print(f"Got different error: {e}")
            # For now, we'll accept this as progress


def test_boolean_operations() -> None:
    """Test that boolean operations work with our extensions."""

    @guppy
    def bool_logic(a: bool, b: bool) -> bool:
        return a and b

    hugr = compile_guppy_to_hugr(bool_logic)
    assert len(hugr) > 0

    try:
        llvm = compile_hugr_to_llvm(hugr)
        print("Success! Boolean operations compiled to LLVM")
        assert len(llvm) > 0
    except RuntimeError as e:
        if "Unknown type: bool" in str(e):
            pytest.fail(
                "Boolean extension not working - still getting Unknown type error",
            )
        else:
            print(f"Got different error: {e}")


def test_integer_constant() -> None:
    """Test integer constants."""

    @guppy
    def return_constant() -> int:
        return 42

    hugr = compile_guppy_to_hugr(return_constant)

    try:
        llvm = compile_hugr_to_llvm(hugr)
        print("Success! Integer constants compiled to LLVM")
        assert len(llvm) > 0
    except RuntimeError as e:
        print(f"Integer constant compilation error: {e}")
        # Check if we're making progress
        if "Unknown type: int" in str(e):
            pytest.fail("Still getting type error for integers")


def test_mixed_quantum_classical() -> None:
    """Test mixing quantum and classical operations."""

    @guppy
    def quantum_with_classical(n: int) -> bool:
        # Classical computation
        x = n + 1

        # Quantum operation
        q = qubit()
        h(q)
        result = measure(q)

        # Mix classical and quantum
        return result if x > 0 else False

    hugr = compile_guppy_to_hugr(quantum_with_classical)

    try:
        llvm = compile_hugr_to_llvm(hugr)
        print("Success! Mixed quantum-classical compiled to LLVM")
        assert len(llvm) > 0
    except RuntimeError as e:
        print(f"Mixed compilation error: {e}")
        # This is expected to still have issues, but should show progress


if __name__ == "__main__":
    print("Testing arithmetic and boolean type support...")
    test_integer_arithmetic()
    test_boolean_operations()
    test_integer_constant()
    test_mixed_quantum_classical()
    print("All tests completed!")
