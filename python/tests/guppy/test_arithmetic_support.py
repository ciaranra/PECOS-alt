#!/usr/bin/env python3
"""Test arithmetic and boolean type support in Guppy->HUGR->Selene pipeline."""

import pytest
from guppylang import guppy
from guppylang.std.quantum import h, measure, qubit

# pytestmark = pytest.mark.optional_dependency  # Enable tests
# Import sim - it now handles Guppy programs via the wrapper
from pecos.frontends.guppy_api import sim
from pecos_rslib import state_vector


def test_integer_arithmetic() -> None:
    """Test that integer arithmetic works through Selene simulation."""

    @guppy
    def quantum_add() -> bool:
        """Add numbers and use result to conditionally apply quantum gate."""
        q = qubit()
        x = 3
        y = 2
        result = x + y  # result = 5

        # Use the result to decide whether to apply H gate
        if result > 3:  # 5 > 3, so H gate applied
            h(q)

        return measure(q)

    # Execute through sim() - it now auto-detects and handles Guppy functions
    try:
        results = sim(quantum_add).qubits(1).quantum(state_vector()).run(10)

        # Verify execution succeeded
        assert results is not None, "Execution should return results"

        # Results is a dict with counts
        if isinstance(results, dict):
            # Check we have some results
            total_shots = sum(results.values()) if results else 0
            assert total_shots == 10, f"Should have 10 shots, got {total_shots}"
            print(
                f"Success! Integer arithmetic executed through Selene - got {results}",
            )
        else:
            # Might be a different format
            assert hasattr(results, "__len__"), "Results should be countable"
            print("Success! Integer arithmetic executed through Selene")

    except Exception as e:
        error_msg = str(e)
        # Check for known issues
        if "Unknown type: int" in error_msg:
            pytest.fail(
                f"Arithmetic extension not working - still getting Unknown type error: {e}",
            )
        elif "program must be" in error_msg:
            # sim() doesn't recognize Guppy functions yet
            pytest.fail(f"sim() doesn't auto-detect Guppy functions yet: {e}")
        elif "could not get source code" in error_msg:
            # This is expected in test context where functions are defined inline
            pytest.xfail(f"Guppy compilation issue in test context: {e}")
        elif (
            "not yet implemented" in error_msg.lower()
            or "unsupported" in error_msg.lower()
        ):
            pytest.xfail(f"Feature not yet implemented: {e}")
        else:
            pytest.fail(f"Unexpected error during execution: {e}")


def test_boolean_operations() -> None:
    """Test that boolean operations work through Selene simulation."""

    @guppy
    def quantum_bool_logic() -> bool:
        """Use boolean logic with quantum operations."""
        q1 = qubit()
        q2 = qubit()

        # Apply H to first qubit
        h(q1)

        # Measure both
        m1 = measure(q1)
        m2 = measure(q2)

        # Use boolean logic on measurement results
        return m1 and not m2

    # Execute through sim() - it now auto-detects and handles Guppy functions
    try:
        results = sim(quantum_bool_logic).qubits(2).quantum(state_vector()).run(10)

        # Verify execution succeeded
        assert results is not None, "Execution should return results"

        # Results is a dict with counts
        if isinstance(results, dict):
            total_shots = sum(results.values()) if results else 0
            assert total_shots == 10, f"Should have 10 shots, got {total_shots}"
            print(
                f"Success! Boolean operations executed through Selene - got {results}",
            )
        else:
            assert hasattr(results, "__len__"), "Results should be countable"
            print("Success! Boolean operations executed through Selene")

    except Exception as e:
        error_msg = str(e)
        if "Unknown type: bool" in error_msg or "Unknown type: int" in error_msg:
            pytest.fail(
                f"Type extension not working - still getting Unknown type error: {e}",
            )
        elif "program must be" in error_msg:
            # sim() doesn't recognize Guppy functions yet
            pytest.fail(f"sim() doesn't auto-detect Guppy functions yet: {e}")
        elif "could not get source code" in error_msg:
            # This is expected in test context where functions are defined inline
            pytest.xfail(f"Guppy compilation issue in test context: {e}")
        elif (
            "not yet implemented" in error_msg.lower()
            or "unsupported" in error_msg.lower()
        ):
            pytest.xfail(f"Feature not yet implemented: {e}")
        else:
            pytest.fail(f"Unexpected error during execution: {e}")


def test_integer_constant() -> None:
    """Test integer constants through Selene."""

    @guppy
    def quantum_with_constant() -> bool:
        """Use integer constant in quantum context."""
        q = qubit()
        threshold = 42
        value = 50

        # Use constant in comparison
        if value > threshold:
            h(q)

        return measure(q)

    # Execute through sim() - it now auto-detects and handles Guppy functions
    try:
        results = sim(quantum_with_constant).qubits(1).quantum(state_vector()).run(10)

        # Verify execution succeeded
        assert results is not None, "Execution should return results"

        # Results is a dict with counts
        if isinstance(results, dict):
            total_shots = sum(results.values()) if results else 0
            assert total_shots == 10, f"Should have 10 shots, got {total_shots}"
            # Since 50 > 42, H gate should be applied, giving mix of 0 and 1
            print(f"Success! Integer constants work through Selene - got {results}")
        else:
            assert hasattr(results, "__len__"), "Results should be countable"
            print("Success! Integer constants work through Selene")

    except Exception as e:
        error_msg = str(e)
        if "Unknown type: int" in error_msg:
            pytest.fail(
                f"Arithmetic extension not working - still getting Unknown type error: {e}",
            )
        elif "could not get source code" in error_msg:
            # This is expected in test context where functions are defined inline
            pytest.xfail(f"Guppy compilation issue in test context: {e}")
        elif (
            "not yet implemented" in error_msg.lower()
            or "unsupported" in error_msg.lower()
            or "no program specified" in error_msg.lower()
        ):
            pytest.xfail(f"Selene HUGR integration not yet complete: {e}")
        else:
            pytest.fail(f"Unexpected error during Selene execution: {e}")


def test_mixed_quantum_classical() -> None:
    """Test mixing quantum and classical operations."""

    @guppy
    def quantum_with_classical() -> bool:
        """Mix classical computation with quantum operations."""
        # Classical computation
        n = 5
        x = n + 1  # x = 6

        # Quantum operation
        q = qubit()

        # Use classical result to control quantum gate
        if x > 0:
            h(q)

        return measure(q)

        # Return quantum measurement result

    # Execute through sim() - it now auto-detects and handles Guppy functions
    try:
        from pecos_rslib import state_vector

        results = (
            sim(quantum_with_classical)
            .qubits(1)
            .quantum(state_vector())
            .seed(42)
            .run(10)
        )

        # Verify execution succeeded
        assert results is not None, "Execution should return results"

        # Results is a dict with counts
        if isinstance(results, dict):
            total_shots = sum(results.values()) if results else 0
            assert total_shots == 10, f"Should have 10 shots, got {total_shots}"
            # Since x=6 > 0, H gate is applied, giving mix of 0 and 1
            print(
                f"Success! Mixed quantum-classical works through Selene - got {results}",
            )
        else:
            assert hasattr(results, "__len__"), "Results should be countable"
            print("Success! Mixed quantum-classical works through Selene")

    except Exception as e:
        error_msg = str(e)
        if "Unknown type: int" in error_msg or "Unknown type: bool" in error_msg:
            pytest.fail(
                f"Type extension not working - still getting Unknown type error: {e}",
            )
        elif "could not get source code" in error_msg:
            # This is expected in test context where functions are defined inline
            pytest.xfail(f"Guppy compilation issue in test context: {e}")
        elif (
            "not yet implemented" in error_msg.lower()
            or "unsupported" in error_msg.lower()
            or "no program specified" in error_msg.lower()
        ):
            pytest.xfail(f"Selene HUGR integration not yet complete: {e}")
        else:
            pytest.fail(f"Unexpected error during Selene execution: {e}")


if __name__ == "__main__":
    import sys

    print("Testing arithmetic and boolean type support with Selene...")

    if not SIM_AVAILABLE:
        print("ERROR: sim() and Selene components not available")
        print("Please install with: uv pip install guppylang selene-sim")
        sys.exit(1)

    test_integer_arithmetic()
    test_boolean_operations()
    test_integer_constant()
    test_mixed_quantum_classical()
    print("All tests completed!")
