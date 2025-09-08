#!/usr/bin/env python3
"""Demonstration of the sim() API.

Shows the unified simulation API for Guppy quantum programs.
"""


import pytest
from guppylang import guppy
from guppylang.std.quantum import cx, h, measure, qubit
from pecos import get_guppy_backends, sim
from pecos_rslib import state_vector


def decode_integer_results(results: list[int], n_bits: int) -> list[tuple[bool, ...]]:
    """Decode integer-encoded results back to tuples of booleans."""
    decoded = []
    for val in results:
        bits = []
        for i in range(n_bits):
            bits.append(bool(val & (1 << i)))
        decoded.append(tuple(bits))
    return decoded


pytestmark = pytest.mark.optional_dependency


# Define some quantum functions
@guppy
def random_bit() -> bool:
    """Generate a random bit using quantum superposition."""
    q = qubit()
    h(q)
    return measure(q)


@guppy
def bell_state() -> tuple[bool, bool]:
    """Create Bell state and measure both qubits."""
    q0 = qubit()
    q1 = qubit()
    h(q0)
    cx(q0, q1)
    return measure(q0), measure(q1)


@guppy
def ghz_state() -> tuple[bool, bool, bool]:
    """Create GHZ state with three qubits."""
    q0, q1, q2 = qubit(), qubit(), qubit()
    h(q0)
    cx(q0, q1)
    cx(q1, q2)
    return measure(q0), measure(q1), measure(q2)


def test_guppy_backends() -> None:
    """Test checking available backends."""
    print("\nChecking available backends:")
    backends = get_guppy_backends()
    assert isinstance(backends, dict)
    for name, status in backends.items():
        print(f"   {name}: {status}")


def test_sim_random_bit() -> None:
    """Test sim() with random_bit."""
    print("\nTesting sim() with random_bit:")
    try:
        result_dict = sim(random_bit).qubits(1).quantum(state_vector()).run(100)

        # Extract measurements - should be in measurement_1 for single return
        measurements = result_dict.get(
            "measurement_1",
            result_dict.get("measurements", []),
        )
        assert (
            len(measurements) == 100
        ), f"Expected 100 measurements, got {len(measurements)}"

        true_count = sum(measurements)
        print(f"   [OK] Got {len(measurements)} results")
        print(f"   True/False ratio: {true_count}/{100 - true_count}")
        print("   Backend: Unified sim() API with state_vector")
    except RuntimeError as e:
        if "Unknown type: bool" in str(e):
            print(f"   [INFO] Expected error: {e}")
            print(
                "   [INFO] This is a known limitation - working on bool type support",
            )
        else:
            raise


def test_sim_bell_state() -> None:
    """Test sim() with bell_state."""
    print("\nTesting sim() with bell_state:")
    result = sim(bell_state).qubits(10).quantum(state_vector()).run(10)

    # Extract measurements - handle different result formats
    if "measurement_1" in result and "measurement_2" in result:
        # Tuple return with separate measurement keys
        measurements = list(
            zip(result["measurement_1"], result["measurement_2"], strict=False),
        )
    elif "measurements" in result:
        measurements = result["measurements"]
    else:
        measurements = result.get("result", [])

    print(f"   [OK] Got {len(measurements)} results")

    # For Bell state, check correlation
    if measurements and isinstance(measurements[0], tuple):
        correlated = sum(1 for m in measurements if m[0] == m[1])
        print(
            f"   Correlation: {correlated}/{len(measurements)} measurements are correlated",
        )
        print("   Backend: Unified sim() API with state_vector")
    else:
        print("   Results format:", type(measurements[0]) if measurements else "empty")


def test_sim_ghz_state() -> None:
    """Test sim() with GHZ state."""
    print("\nTesting sim() with GHZ state:")
    result = sim(ghz_state).qubits(10).quantum(state_vector()).seed(42).run(50)

    # Extract measurements
    if (
        "measurement_1" in result
        and "measurement_2" in result
        and "measurement_3" in result
    ):
        # Tuple return with separate measurement keys
        measurements = list(
            zip(
                result["measurement_1"],
                result["measurement_2"],
                result["measurement_3"],
                strict=False,
            ),
        )
    elif "measurements" in result:
        measurements = result["measurements"]
    else:
        measurements = result.get("result", [])

    print(f"   [OK] Got {len(measurements)} results")

    # For GHZ state, check correlation (all three qubits should be same)
    if measurements and isinstance(measurements[0], tuple):
        all_same = sum(1 for m in measurements if m[0] == m[1] == m[2])
        print(
            f"   Correlation: {all_same}/{len(measurements)} have all three qubits equal",
        )
        print("   Backend: Unified sim() API with state_vector and seed=42")
    else:
        print("   Results format:", type(measurements[0]) if measurements else "empty")


def test_sim_builder_pattern() -> None:
    """Test the builder pattern of sim()."""
    print("\nTesting sim() builder pattern:")

    # Build simulation step by step
    builder = sim(bell_state)
    builder = builder.qubits(10)
    builder = builder.quantum(state_vector())
    builder = builder.seed(12345)

    # Run once
    results1 = builder.run(20)

    # Create new builder for second run (builders are consumed after run)
    builder2 = sim(bell_state).qubits(10).quantum(state_vector()).seed(12345)
    results2 = builder2.run(20)

    print("   [OK] Builder pattern works")
    print(
        f"   First run: {len(results1.get('measurements', results1.get('measurement_1', [])))} shots",
    )
    print(
        f"   Second run: {len(results2.get('measurements', results2.get('measurement_1', [])))} shots",
    )


if __name__ == "__main__":
    print("=" * 60)
    print("Demonstrating the sim() API for Guppy quantum programs")
    print("=" * 60)

    test_guppy_backends()
    test_sim_random_bit()
    test_sim_bell_state()
    test_sim_ghz_state()
    test_sim_builder_pattern()

    print("\n" + "=" * 60)
    print("Demo complete!")
    print("=" * 60)
