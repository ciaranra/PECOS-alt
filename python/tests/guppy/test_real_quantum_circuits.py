#!/usr/bin/env python3
"""Test real quantum circuits through the Guppy->HUGR->Selene->ByteMessage pipeline."""

import numpy as np
import pytest
from guppylang import guppy
from guppylang.std.quantum import cx, h, measure, qubit, ry, rz, x, z

pytestmark = pytest.mark.optional_dependency

# Import sim
from pecos_rslib.sim import sim


def test_bell_state_preparation() -> None:
    """Test Bell state preparation and measurement."""

    @guppy
    def prepare_bell_state() -> tuple[bool, bool]:
        """Prepare a Bell state |Φ+⟩ = (|00⟩ + |11⟩)/√2."""
        q1 = qubit()
        q2 = qubit()

        # Create Bell state
        h(q1)
        cx(q1, q2)

        # Measure both qubits
        m1 = measure(q1)
        m2 = measure(q2)

        return (m1, m2)

    # Run simulation
    results = sim(prepare_bell_state).qubits(2).seed(42).run(1000)

    # Bell state should give correlated results: either (0,0) or (1,1)
    assert results is not None, "Should get results"
    print(f"Bell state results type: {type(results)}")
    print(f"Bell state results: {results}")

    # Count outcomes
    both_zero = 0
    both_one = 0
    anti_correlated = 0

    # Handle different result formats
    if hasattr(results, "counts"):
        # ShotMap format
        counts = results.counts()
        for outcome, count in counts.items():
            if outcome == (False, False) or outcome == "00":
                both_zero += count
            elif outcome == (True, True) or outcome == "11":
                both_one += count
            else:
                anti_correlated += count
    elif hasattr(results, "__iter__"):
        # ShotVec format - iterate through individual shots
        for shot in results:
            print(f"Shot: {shot}")  # Debug: see what each shot looks like
            # For now, just count
            both_zero += 1
    elif isinstance(results, dict):
        for outcome, count in results.items():
            if outcome == (False, False) or outcome == "00":
                both_zero += count
            elif outcome == (True, True) or outcome == "11":
                both_one += count
            else:
                anti_correlated += count

    # Bell state should only produce correlated outcomes
    assert (
        anti_correlated == 0
    ), f"Bell state should not produce anti-correlated outcomes, got {anti_correlated}"
    assert both_zero > 0, "Should see |00⟩ outcomes"
    assert both_one > 0, "Should see |11⟩ outcomes"

    # Should be roughly 50/50 split
    total = both_zero + both_one
    assert (
        0.4 < both_zero / total < 0.6
    ), f"Should be ~50% |00⟩, got {both_zero / total}"
    assert 0.4 < both_one / total < 0.6, f"Should be ~50% |11⟩, got {both_one / total}"

    print(f"Bell state test passed: |00⟩={both_zero}, |11⟩={both_one}")


def test_ghz_state() -> None:
    """Test 3-qubit GHZ state preparation."""

    @guppy
    def prepare_ghz_state() -> tuple[bool, bool, bool]:
        """Prepare a GHZ state |GHZ⟩ = (|000⟩ + |111⟩)/√2."""
        q1 = qubit()
        q2 = qubit()
        q3 = qubit()

        # Create GHZ state
        h(q1)
        cx(q1, q2)
        cx(q1, q3)

        # Measure all qubits
        m1 = measure(q1)
        m2 = measure(q2)
        m3 = measure(q3)

        return (m1, m2, m3)

    # Run simulation
    results = sim(prepare_ghz_state).qubits(3).seed(42).run(1000)

    assert results is not None, "Should get results"

    # GHZ state should give either all 0s or all 1s
    all_zero = 0
    all_one = 0
    other = 0

    if isinstance(results, dict):
        for outcome, count in results.items():
            if outcome == (False, False, False) or outcome == "000":
                all_zero += count
            elif outcome == (True, True, True) or outcome == "111":
                all_one += count
            else:
                other += count

    # GHZ state should only produce |000⟩ or |111⟩
    assert other == 0, f"GHZ state should not produce mixed outcomes, got {other}"
    assert all_zero > 0, "Should see |000⟩ outcomes"
    assert all_one > 0, "Should see |111⟩ outcomes"

    print(f"GHZ state test passed: |000⟩={all_zero}, |111⟩={all_one}")


def test_quantum_phase_kickback() -> None:
    """Test quantum phase kickback circuit."""

    @guppy
    def phase_kickback_circuit() -> tuple[bool, bool]:
        """Demonstrate phase kickback with controlled-Z gate."""
        control = qubit()
        target = qubit()

        # Put control in superposition
        h(control)

        # Put target in |1⟩ state
        x(target)

        # Apply controlled-Z (phase kickback occurs)
        # Since we don't have cz directly, use the equivalence: CZ = H·CX·H
        h(target)
        cx(control, target)
        h(target)

        # Measure in X basis for control (apply H before measure)
        h(control)
        m1 = measure(control)

        # Measure target in Z basis
        m2 = measure(target)

        return (m1, m2)

    # Run simulation
    results = sim(phase_kickback_circuit).qubits(2).seed(42).run(1000)

    assert results is not None, "Should get results"

    # The control qubit should measure |1⟩ in X basis (due to phase kickback)
    # The target should remain in |1⟩
    control_one_count = 0
    target_one_count = 0
    total = 0

    if isinstance(results, dict):
        for outcome, count in results.items():
            total += count
            if isinstance(outcome, tuple):
                if outcome[0] or (isinstance(outcome, str) and outcome[0] == "1"):
                    control_one_count += count
                if outcome[1] or (isinstance(outcome, str) and outcome[1] == "1"):
                    target_one_count += count

    # Control should be predominantly |1⟩ due to phase kickback
    assert (
        control_one_count / total > 0.9
    ), f"Control should be ~100% |1⟩ after phase kickback, got {control_one_count / total}"
    # Target should remain |1⟩
    assert (
        target_one_count / total > 0.9
    ), f"Target should remain |1⟩, got {target_one_count / total}"

    print(
        f"Phase kickback test passed: control |1⟩={control_one_count}/{total}, target |1⟩={target_one_count}/{total}",
    )


def test_quantum_interference() -> None:
    """Test quantum interference in a simple interferometer."""

    @guppy
    def quantum_interferometer() -> bool:
        """Create quantum interference using H gates."""
        q = qubit()

        # First H gate - creates superposition
        h(q)

        # Phase shift of π
        z(q)

        # Second H gate - creates interference
        h(q)

        # Should measure |1⟩ due to destructive interference
        return measure(q)

    # Run simulation
    results = sim(quantum_interferometer).qubits(1).seed(42).run(1000)

    assert results is not None, "Should get results"

    # Due to interference, should measure |1⟩ ~100% of the time
    one_count = 0
    total = 0

    if isinstance(results, dict):
        for outcome, count in results.items():
            total += count
            if outcome or outcome == 1 or outcome == "1":
                one_count += count

    assert (
        one_count / total > 0.95
    ), f"Should measure |1⟩ due to interference, got {one_count / total}"

    print(f"Interference test passed: |1⟩={one_count}/{total}")


def test_rotation_gates() -> None:
    """Test rotation gates with specific angles."""

    @guppy
    def rotation_circuit() -> bool:
        """Test Y and Z rotations."""
        q = qubit()

        # Rotate around Y axis by π/2 (creates equal superposition)
        ry(q, np.pi / 2)

        # Rotate around Z axis by π/4 (adds phase)
        rz(q, np.pi / 4)

        # Measure
        return measure(q)

    # Run simulation
    results = sim(rotation_circuit).qubits(1).seed(42).run(1000)

    assert results is not None, "Should get results"

    # After Ry(π/2), should be in equal superposition
    # Rz just adds phase, doesn't change measurement probabilities
    zero_count = 0
    one_count = 0

    if isinstance(results, dict):
        for outcome, count in results.items():
            if not outcome or outcome == 0 or outcome == "0":
                zero_count += count
            else:
                one_count += count

    total = zero_count + one_count
    # Should be roughly 50/50 after Ry(π/2)
    assert (
        0.4 < zero_count / total < 0.6
    ), f"Should be ~50% |0⟩ after Ry(π/2), got {zero_count / total}"
    assert (
        0.4 < one_count / total < 0.6
    ), f"Should be ~50% |1⟩ after Ry(π/2), got {one_count / total}"

    print(f"Rotation test passed: |0⟩={zero_count}, |1⟩={one_count}")


if __name__ == "__main__":
    print("Testing real quantum circuits through Selene bridge...")

    test_bell_state_preparation()
    test_ghz_state()
    test_quantum_phase_kickback()
    test_quantum_interference()
    test_rotation_gates()

    print("\nAll real quantum circuit tests passed!")
