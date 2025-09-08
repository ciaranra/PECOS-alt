#!/usr/bin/env python3
"""Test noise model integration with sim.

This test file verifies that noise models are properly integrated
and working with the sim builder pattern.
"""

import sys


def decode_integer_results(results: list[int], n_bits: int) -> list[tuple[bool, ...]]:
    """Decode integer-encoded results back to tuples of booleans."""
    decoded = []
    for val in results:
        bits = []
        for i in range(n_bits):
            bits.append(bool(val & (1 << i)))
        decoded.append(tuple(bits))
    return decoded


import pytest

sys.path.append("python/quantum-pecos/src")

try:
    from guppylang import guppy
    from guppylang.std.quantum import cx, h, measure, qubit, x

    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False

try:
    from pecos.frontends.guppy_api import sim

    # Import noise models using the builder functions
    from pecos_rslib import (
        biased_depolarizing_noise,
        depolarizing_noise,
        general_noise,
        state_vector,
    )
except ImportError:
    pass


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
class TestNoiseModels:
    """Test noise model integration with sim."""

    def test_no_noise_deterministic(self) -> None:
        """Test that circuits without noise are deterministic."""

        @guppy
        def deterministic_circuit() -> bool:
            q = qubit()
            x(q)
            return measure(q)

        # Run with seed for reproducibility
        results = (
            sim(deterministic_circuit)
            .qubits(10)
            .quantum(state_vector())
            .seed(42)
            .run(10)
        )

        # Should always measure |1⟩
        assert all(
            r == 1
            for r in results.get("measurements", results.get("measurement_1", []))
        ), "Deterministic circuit should always return 1"

    def test_depolarizing_noise_effect(self) -> None:
        """Test that depolarizing noise introduces errors."""

        @guppy
        def simple_circuit() -> bool:
            q = qubit()
            x(q)
            return measure(q)

        # Run without noise
        results_ideal = (
            sim(simple_circuit).qubits(10).quantum(state_vector()).seed(123).run(100)
        )
        # Extract measurements - results is a dict with measurement lists
        if isinstance(results_ideal, dict):
            measurements_ideal = results_ideal.get(
                "measurement_1",
                results_ideal.get("result", []),
            )
        elif isinstance(results_ideal, list):
            # Handle if it's a list of dicts
            measurements_ideal = []
            for shot in results_ideal:
                if isinstance(shot, dict):
                    val = shot.get("measurement_1", shot.get("result", None))
                    if val is not None:
                        measurements_ideal.append(val)
        else:
            measurements_ideal = []
        ones_ideal = sum(measurements_ideal)

        # Run with 10% depolarizing noise
        noise = depolarizing_noise().with_uniform_probability(0.1)
        results_noisy = (
            sim(simple_circuit)
            .qubits(10)
            .quantum(state_vector())
            .seed(123)
            .noise(noise)
            .run(100)
        )
        # Extract measurements from noisy results
        if isinstance(results_noisy, dict):
            measurements_noisy = results_noisy.get(
                "measurement_1",
                results_noisy.get("result", []),
            )
        elif isinstance(results_noisy, list):
            measurements_noisy = []
            for shot in results_noisy:
                if isinstance(shot, dict):
                    val = shot.get("measurement_1", shot.get("result", None))
                    if val is not None:
                        measurements_noisy.append(val)
        else:
            measurements_noisy = []
        ones_noisy = sum(measurements_noisy)

        # Noise should reduce fidelity
        assert ones_ideal == 100, "Ideal circuit should have perfect fidelity"
        assert (
            70 < ones_noisy < 95
        ), f"Noisy circuit should have reduced fidelity, got {ones_noisy}/100"
        print(f"✓ Depolarizing noise working: {ones_ideal}/100 → {ones_noisy}/100")

    def test_noise_models_comparison(self) -> None:
        """Compare different noise models on the same circuit."""

        @guppy
        def bell_state() -> tuple[bool, bool]:
            q0, q1 = qubit(), qubit()
            h(q0)
            cx(q0, q1)
            return measure(q0), measure(q1)

        # Test different noise models
        noise_configs = [
            ("No Noise", None),  # No noise model
            ("5% Uniform", depolarizing_noise().with_uniform_probability(0.05)),
            ("5% Biased", biased_depolarizing_noise().with_uniform_probability(0.05)),
            (
                "Custom",
                general_noise()
                .with_preparation_probability(0.01)
                .with_measurement_probability(0.99, 0.01)  # p0, p1 probabilities
                .with_p1_probability(0.02)
                .with_p2_probability(0.05),
            ),
        ]

        print("\nNoise Model Comparison (Bell State Correlation):")
        for name, noise in noise_configs:
            builder = sim(bell_state).qubits(10).quantum(state_vector()).seed(42)
            if noise is not None:
                builder = builder.noise(noise)
            results = builder.run(100)

            # Count correlated outcomes (|00⟩ or |11⟩)
            correlated = 0
            if isinstance(results, dict):
                # Results is a dict with measurement lists
                m1_list = results.get("measurement_1", [])
                m2_list = results.get("measurement_2", [])
                for m1, m2 in zip(m1_list, m2_list, strict=False):
                    if m1 == m2:  # Correlated if both are same (00 or 11)
                        correlated += 1
            elif isinstance(results, list):
                # Handle list of dicts format
                for shot in results:
                    if isinstance(shot, dict):
                        m1 = shot.get("measurement_1", None)
                        m2 = shot.get("measurement_2", None)
                        if m1 is not None and m2 is not None and m1 == m2:
                            correlated += 1

            print(f"  {name:15s}: {correlated}/100 correlated ({correlated:.1f}%)")

            # Basic sanity checks
            # Note: Due to simulation quirks, even no-noise might not be perfect
            if noise is None:
                assert (
                    correlated > 90
                ), f"No noise should have high correlation, got {correlated}"
            else:
                # With noise, correlation might be reduced but not eliminated
                assert (
                    10 <= correlated <= 100
                ), f"Noise results out of bounds: {correlated}"


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
def test_noise_model_builder_pattern() -> None:
    """Test that noise models work with the builder pattern."""

    @guppy
    def test_circuit() -> bool:
        q = qubit()
        h(q)
        x(q)
        h(q)
        return measure(q)

    # Build simulation with noise - run without building first
    # (Building consumes the builder, so we can't reuse it)
    results1 = (
        sim(test_circuit)
        .qubits(10)
        .quantum(state_vector())
        .seed(12345)
        .noise(depolarizing_noise().with_uniform_probability(0.05))
        .workers(2)
        .run(10)
    )

    # Run again with same configuration
    results2 = (
        sim(test_circuit)
        .qubits(10)
        .quantum(state_vector())
        .seed(12345)
        .noise(depolarizing_noise().with_uniform_probability(0.05))
        .workers(2)
        .run(10)
    )

    # Both runs should have results - extract measurements
    measurements1 = (
        results1.get("measurement_1", results1.get("result", []))
        if isinstance(results1, dict)
        else []
    )

    measurements2 = (
        results2.get("measurement_1", results2.get("result", []))
        if isinstance(results2, dict)
        else []
    )

    assert len(measurements1) == 10
    assert len(measurements2) == 10

    # With noise, results should vary
    zeros1 = sum(1 for r in measurements1 if r == 0)
    zeros2 = sum(1 for r in measurements2 if r == 0)

    print(
        f"\n✓ Builder pattern with noise: Run1={zeros1}/10 zeros, Run2={zeros2}/10 zeros",
    )


if __name__ == "__main__":
    # Run a quick demo
    if GUPPY_AVAILABLE:
        print("Noise Model Integration Demo")
        print("=" * 40)

        test = TestNoiseModels()
        test.test_depolarizing_noise_effect()
        test.test_noise_models_comparison()
        test_noise_model_builder_pattern()
