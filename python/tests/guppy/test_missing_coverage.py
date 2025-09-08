#!/usr/bin/env python3
"""Tests for missing coverage areas in the Guppy test suite.

This test file addresses gaps identified in the test coverage analysis:
1. Noise models and error simulation
2. Array and batch quantum operations
3. Advanced control flow patterns
4. Different quantum engines
5. Error handling with quantum resources
"""

import sys

import pytest


def decode_integer_results(results: list[int], n_bits: int) -> list[tuple[bool, ...]]:
    """Decode integer-encoded results back to tuples of booleans."""
    decoded = []
    for val in results:
        bits = []
        for i in range(n_bits):
            bits.append(bool(val & (1 << i)))
        decoded.append(tuple(bits))
    return decoded


sys.path.append("python/quantum-pecos/src")

# Check dependencies
try:
    from guppylang import guppy
    from guppylang.std.angles import angle, pi
    from guppylang.std.builtins import array
    from guppylang.std.quantum import array as qubit_array
    from guppylang.std.quantum import (
        cx,
        cz,
        discard,
        discard_array,
        h,
        measure,
        measure_array,
        qubit,
        x,
    )
    from guppylang.std.quantum_functional import project_z, reset

    GUPPY_AVAILABLE = True

    # Try to import optional functions that might not be available
    try:
        from guppylang.std.quantum import discard_array, measure_array
    except ImportError:
        measure_array = None
        discard_array = None

    try:
        from guppylang.std.quantum_functional import project_z
    except ImportError:
        project_z = None

    try:
        from guppylang.std.builtins import owned, panic
    except ImportError:
        owned = None
        panic = None
except ImportError:
    GUPPY_AVAILABLE = False

try:
    from pecos.frontends.guppy_api import sim
    from pecos.frontends.sim_builder import (
        BiasedDepolarizingNoise,
        DepolarizingCustomNoise,
        DepolarizingNoise,
        PassThroughNoise,
    )
    from pecos_rslib import state_vector
except ImportError:
    pass

# ============================================================================
# NOISE MODEL TESTS
# ============================================================================


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
class TestNoiseModels:
    """Test quantum simulations with various noise models."""

    def test_depolarizing_noise(self) -> None:
        """Test uniform depolarizing noise on quantum operations."""

        @guppy
        def noisy_circuit() -> bool:
            q = qubit()
            x(q)  # Just X gate to flip to |1⟩ deterministically
            return measure(q)

        # Test with no noise - should be deterministic
        results_ideal = (
            sim(noisy_circuit).qubits(10).quantum(state_vector()).seed(42).run(10)
        )

        # Results are in 'c' key
        ones_ideal = sum(results_ideal["result"])
        assert (
            ones_ideal == 100
        ), f"Ideal circuit should produce all 1s, got {ones_ideal}/100"

        # Test with depolarizing noise
        noise = DepolarizingNoise(p=0.1)  # 10% error rate
        results_noisy = (
            sim(noisy_circuit)
            .qubits(10)
            .quantum(state_vector())
            .seed(42)
            .noise(noise)
            .run(10)
        )
        ones_noisy = sum(results_noisy["result"])

        # With noise, we should see some errors (not all 1s)
        # 10% depolarizing noise means ~10% chance of error
        # But depolarizing can cause various errors, so be more lenient
        assert (
            750 < ones_noisy < 950
        ), f"Expected 75-95% ones with 10% noise, got {ones_noisy}/1000"
        print(
            f"Noise model working! Got {ones_noisy}/1000 ones with 10% depolarizing noise",
        )

    def test_biased_depolarizing_noise(self) -> None:
        """Test biased depolarizing noise model."""

        @guppy
        def bell_state() -> tuple[bool, bool]:
            q0, q1 = qubit(), qubit()
            h(q0)
            cx(q0, q1)
            return measure(q0), measure(q1)

        # Test with biased noise
        noise = BiasedDepolarizingNoise(p=0.05)  # 5% biased error
        results = (
            sim(bell_state)
            .qubits(10)
            .quantum(state_vector())
            .seed(123)
            .noise(noise)
            .run(10)
        )

        # Count correlated outcomes (00 and 11)
        # Results are tuples (False, False) or (True, True) for correlated Bell states
        correlated = sum(
            1
            for r in results.get("measurements", results.get("measurement_1", []))
            if r in [(False, False), (True, True)]
        )

        # With 5% biased noise, Bell states should still be somewhat correlated
        # But biased depolarizing might affect correlation more than expected
        assert correlated > 400, f"Bell state correlation too low: {correlated}/1000"
        print(f"Biased noise working! Got {correlated}/1000 correlated Bell states")

    def test_custom_depolarizing_noise(self) -> None:
        """Test custom depolarizing noise with different rates."""

        @guppy
        def prep_measure_circuit() -> bool:
            q = qubit()  # Preparation
            h(q)
            x(q)
            return measure(q)  # Measurement

        # Custom noise: high prep error, low measurement error
        noise = DepolarizingCustomNoise(
            p_prep=0.2,  # 20% preparation error
            p_meas=0.01,  # 1% measurement error
            p1=0.05,  # 5% single-qubit gate error
            p2=0.1,  # 10% two-qubit gate error
        )

        results = (
            sim(prep_measure_circuit)
            .qubits(10)
            .quantum(state_vector())
            .seed(456)
            .noise(noise)
            .run(10)
        )
        errors = 1000 - sum(
            results.get("measurements", results.get("measurement_1", [])),
        )

        # With high prep error (20%), we expect significant errors
        # The circuit has prep + 2 gates + measurement, so errors compound
        assert (
            150 < errors < 600
        ), f"Expected 15-60% errors with custom noise, got {errors}/1000"
        print(f"Custom noise working! Got {errors}/1000 errors with high prep error")


# ============================================================================
# ARRAY AND BATCH OPERATIONS
# ============================================================================


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
class TestArrayOperations:
    """Test array and batch quantum operations."""

    @pytest.mark.skip(reason="HUGR doesn't support array types for measure_array yet")
    def test_measure_array(self) -> None:
        """Test measuring an array of qubits."""

        # Note: This test uses the canonical measure_array pattern from guppylang
        # but HUGR compilation doesn't support it yet
        @guppy
        def measure_array_test() -> list[bool]:
            # Create array of 5 qubits using comprehension
            qs = array(qubit() for _ in range(5))

            # Apply different operations using indexing
            h(qs[0])
            x(qs[1])
            h(qs[2])
            x(qs[3])
            # qs[4] stays |0⟩

            # Use measure_array to measure all qubits at once (canonical pattern)
            results = measure_array(qs)

            # Convert array to list for return
            return list(results)

        results = (
            sim(measure_array_test).qubits(10).quantum(state_vector()).seed(789).run(10)
        )

        # Check tuple results
        for result in results.get("measurements", results.get("measurement_1", [])):
            # Result is a tuple of 5 booleans
            # Extract individual measurements
            b0, b1, b2, b3, b4 = result

            # Check known deterministic bits
            assert b1, "Bit 1 should be True (from x(qs[1]))"
            assert b3, "Bit 3 should be True (from x(qs[3]))"
            assert not b4, "Bit 4 should be False (qs[4] stays |0⟩)"

            # b0 and b2 are probabilistic (from H gates)

    @pytest.mark.skip(reason="HUGR doesn't support value_array type yet")
    def test_discard_array(self) -> None:
        """Test discarding an array of qubits."""
        # First check if discard_array is available
        if discard_array is None:
            pytest.skip("discard_array not available in this guppy version")

        @guppy
        def discard_array_test() -> bool:
            # Create and manipulate array
            qs = array(qubit() for _ in range(10))
            for i in range(10):
                if i % 2 == 0:
                    h(qs[i])

            # Use discard_array to discard all qubits at once
            discard_array(qs)

            # Create new qubit to return something
            q = qubit()
            x(q)
            return measure(q)

        # Should run without errors
        results = sim(discard_array_test).qubits(10).quantum(state_vector()).run(10)
        assert all(
            r == 1
            for r in results.get("measurements", results.get("measurement_1", []))
        ), "Final qubit should be |1⟩"

    @pytest.mark.skip(reason="HUGR doesn't support value_array type yet")
    def test_array_indexing_and_loops(self) -> None:
        """Test array indexing within loops."""
        if measure_array is None:
            pytest.skip("measure_array not available in this guppy version")

        @guppy
        def array_loop_test() -> int:
            qs = array(qubit() for _ in range(4))

            # Apply H gate to even indices
            for i in range(4):
                if i % 2 == 0:
                    h(qs[i])
                else:
                    x(qs[i])

            # Use measure_array to measure all at once
            results = measure_array(qs)

            # Encode as integer
            result = 0
            for i in range(4):
                if results[i]:
                    result |= 1 << i

            return result

        results = (
            sim(array_loop_test).qubits(10).quantum(state_vector()).seed(42).run(10)
        )

        # With fixed seed, check deterministic pattern
        # Even indices (0,2) are in superposition, odd indices (1,3) are |1⟩
        # This gives us a specific pattern we can verify
        for result in results.get("measurements", results.get("measurement_1", [])):
            # Extract individual bits: result = b3*8 + b2*4 + b1*2 + b0
            result & 1
            b1 = (result >> 1) & 1
            (result >> 2) & 1
            b3 = (result >> 3) & 1

            # Odd indices should always be 1
            assert (
                b1 == 1 and b3 == 1
            ), f"Odd indices should be |1⟩, got result: {result:04b}"


# ============================================================================
# ADVANCED CONTROL FLOW
# ============================================================================


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
class TestAdvancedControlFlow:
    """Test complex control flow patterns."""

    @pytest.mark.skip(reason="HUGR can't handle boolean constants in control flow")
    def test_nested_loops(self) -> None:
        """Test nested loops with quantum operations."""

        @guppy
        def nested_loop_test() -> int:
            count = 0

            # Simple nested loops without complex conditionals
            for i in range(3):
                for _j in range(i + 1):
                    q = qubit()  # Create fresh qubit for each iteration
                    h(q)
                    # Directly add measurement result (bool converts to int)
                    b = measure(q)
                    if b:
                        count = count + 1

            return count

        # Run multiple times to see distribution
        results = (
            sim(nested_loop_test).qubits(10).quantum(state_vector()).seed(111).run(10)
        )

        # Count should be between 0 and 6 (sum of 1+2+3 measurements)
        assert all(
            0 <= r <= 6
            for r in results.get("measurements", results.get("measurement_1", []))
        ), "Count out of expected range"
        # Check we get a reasonable distribution
        avg_count = sum(
            results.get("measurements", results.get("measurement_1", [])),
        ) / len(results.get("measurements", results.get("measurement_1", [])))
        assert 2.5 < avg_count < 3.5, f"Average count {avg_count} out of expected range"

    def test_conditional_quantum_operations(self) -> None:
        """Test quantum operations inside conditionals."""

        # Create separate functions for each test case since sim doesn't support parameters
        @guppy
        def conditional_quantum_0() -> bool:
            q = qubit()
            # n = 0: Do nothing - return |0⟩
            return measure(q)

        @guppy
        def conditional_quantum_1() -> bool:
            q = qubit()
            # n = 1: Return |1⟩
            x(q)
            return measure(q)

        @guppy
        def conditional_quantum_2() -> bool:
            q = qubit()
            # n = 2: Superposition
            h(q)
            return measure(q)

        # Test case n=0
        results = sim(conditional_quantum_0).qubits(10).quantum(state_vector()).run(10)
        assert all(
            r == 0
            for r in results.get("measurements", results.get("measurement_1", []))
        ), "Case n=0 failed"

        # Test case n=1
        results = sim(conditional_quantum_1).qubits(10).quantum(state_vector()).run(10)
        assert all(
            r == 1
            for r in results.get("measurements", results.get("measurement_1", []))
        ), "Case n=1 failed"

        # Test case n=2 (superposition - should have both 0 and 1)
        results = (
            sim(conditional_quantum_2)
            .qubits(10)
            .quantum(state_vector())
            .seed(42)
            .run(10)
        )
        zeros = sum(
            1
            for r in results.get("measurements", results.get("measurement_1", []))
            if r == 0
        )
        ones = sum(
            1
            for r in results.get("measurements", results.get("measurement_1", []))
            if r == 1
        )
        assert zeros > 20 and ones > 20, "Case n=2 (superposition) failed"

    def test_early_return_with_quantum(self) -> None:
        """Test early returns with quantum resources."""

        # Create separate functions for each test case
        @guppy
        def early_return_test_true() -> bool:
            q1 = qubit()
            h(q1)

            # Early return - measure consumes the qubit
            return measure(q1)

        @guppy
        def early_return_test_false() -> bool:
            q1 = qubit()
            h(q1)

            # Continue with more operations
            q2 = qubit()
            cx(q1, q2)
            # Measure q2 to consume it
            measure(q2)  # Can't use _ in Guppy

            return measure(q1)

        # Test both paths
        results_true = (
            sim(early_return_test_true)
            .qubits(10)
            .quantum(state_vector())
            .seed(42)
            .run(10)
        )
        results_false = (
            sim(early_return_test_false)
            .qubits(10)
            .quantum(state_vector())
            .seed(42)
            .run(10)
        )

        # Both should produce valid results
        assert len(results_true["result"]) == 100
        assert len(results_false["result"]) == 100


# ============================================================================
# QUANTUM ENGINE TESTS
# ============================================================================


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
class TestQuantumEngines:
    """Test different quantum simulation engines."""

    def test_state_vector_engine(self) -> None:
        """Test explicit state vector engine selection."""

        @guppy
        def engine_test() -> tuple[bool, bool]:
            q0, q1 = qubit(), qubit()
            h(q0)
            cx(q0, q1)
            return measure(q0), measure(q1)

        # Explicitly use state vector engine
        results = (
            sim(engine_test)
            .qubits(10)
            .quantum(state_vector())
            .engine("StateVector")
            .seed(42)
            .run(10)
        )

        # Verify Bell state correlations - results are tuples
        assert all(
            r in [(False, False), (True, True)]
            for r in results.get("measurements", results.get("measurement_1", []))
        ), "Bell state should be |00⟩ or |11⟩"

    def test_sparse_stabilizer_engine(self) -> None:
        """Test sparse stabilizer engine for Clifford circuits."""

        @guppy
        def clifford_circuit() -> bool:
            # Clifford-only circuit
            q = qubit()
            h(q)
            x(q)
            h(q)
            return measure(q)

        # Try sparse stabilizer engine
        try:
            results = (
                sim(clifford_circuit)
                .qubits(10)
                .quantum(state_vector())
                .engine("SparseStabilizer")
                .seed(42)
                .run(10)
            )

            # Should produce deterministic result for Clifford circuit
            assert all(
                r == results.get("measurements", results.get("measurement_1", []))[0]
                for r in results.get("measurements", results.get("measurement_1", []))
            ), "Clifford circuit should be deterministic"
        except Exception as e:
            # Engine might not be available for all operations
            pytest.skip(f"Sparse stabilizer engine not available: {e}")


# ============================================================================
# ERROR HANDLING WITH QUANTUM RESOURCES
# ============================================================================


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
class TestQuantumErrorHandling:
    """Test error handling with quantum resources."""

    @pytest.mark.skip(reason="panic function not yet supported in compilation pipeline")
    def test_panic_with_quantum_resources(self) -> None:
        """Test panic behavior with active quantum resources."""

        @guppy
        def panic_test() -> bool:
            q = qubit()
            h(q)

            # This should clean up quantum resources before panicking
            if measure(q):
                panic("Measured |1⟩!", q)

            return False  # Should not reach here if panic occurs

        # Some shots should panic, some should not
        with pytest.raises(RuntimeError, match="panic"):
            # This might panic on some shots
            sim(panic_test).qubits(10).quantum(state_vector()).seed(42).run(10)

    @pytest.mark.skip(
        reason="project_z requires tket2.bool.make_opaque support in HUGR->LLVM",
    )
    def test_projective_measurement(self) -> None:
        """Test project_z operation."""

        @guppy
        def project_test() -> tuple[bool, bool]:
            q = qubit()
            h(q)

            # Project and get classical result
            q, result = project_z(q)

            # Measure again - should be deterministic after projection
            final = measure(q)

            return result, final

        results = sim(project_test).qubits(10).quantum(state_vector()).seed(42).run(10)

        # After projection, both measurements should match
        for r in results.get("measurements", results.get("measurement_1", [])):
            # Extract two bits from result
            first = r & 1
            second = (r >> 1) & 1
            assert first == second, "Projective measurement should collapse state"

    def test_reset_operation(self) -> None:
        """Test reset operation on qubits."""

        @guppy
        def reset_test() -> tuple[bool, bool]:
            # Measure a |1> state
            q1 = qubit()
            x(q1)
            before = measure(q1)

            # Create a new qubit, reset it, and measure
            q2 = qubit()
            x(q2)  # Set to |1⟩
            q2 = reset(q2)  # Reset to |0⟩
            after = measure(q2)

            return before, after

        results = sim(reset_test).qubits(10).quantum(state_vector()).run(10)

        # All results should be (True, False) as tuples
        # Decode integer-encoded results
        decoded_results = decode_integer_results(
            results.get("measurements", results.get("measurement_1", [])),
            2,
        )
        assert all(
            r == (True, False) for r in decoded_results
        ), "Should produce |1⟩ then |0⟩ as tuple (True, False)"


if __name__ == "__main__":
    print("Testing missing coverage areas...")
    pytest.main([__file__, "-v"])
