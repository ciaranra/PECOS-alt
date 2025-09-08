"""Test to check if tuple values are in wrong order."""

from guppylang import guppy
from guppylang.std.quantum import measure, qubit, x, y, z
from pecos.frontends.guppy_api import sim
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


def test_tuple_order() -> None:
    """Test if measurements are returned in the wrong order."""

    @guppy
    def tuple_order_test() -> tuple[bool, bool, bool, bool]:
        # Create predictable pattern: F, T, F, T
        q1 = qubit()  # |0⟩ -> False
        r1 = measure(q1)

        q2 = qubit()
        x(q2)  # |1⟩ -> True
        r2 = measure(q2)

        q3 = qubit()  # |0⟩ -> False
        r3 = measure(q3)

        q4 = qubit()
        x(q4)  # |1⟩ -> True
        r4 = measure(q4)

        return r1, r2, r3, r4

    results = sim(tuple_order_test).qubits(4).quantum(state_vector()).run(1)
    # Decode integer-encoded results
    decoded_results = decode_integer_results(
        results.get("measurements", results.get("measurement_1", [])),
        4,
    )
    val = decoded_results[0]
    r1, r2, r3, r4 = val

    print("Expected: (False, True, False, True)")
    print(f"Got:      ({r1}, {r2}, {r3}, {r4})")

    # Check various ordering hypotheses
    if (r1, r2, r3, r4) == (False, True, False, True):
        print("✓ Correct order!")
    elif (r1, r2, r3, r4) == (False, True, True, False):
        print("✗ Values appear to be r1, r2, r4, r3 (last two swapped)")
    elif (r1, r2, r3, r4) == (True, False, True, False):
        print("✗ Values appear to be r2, r1, r4, r3 (pairs swapped)")
    elif (r1, r2, r3, r4) == (False, False, True, True):
        print("✗ Values appear to be r1, r3, r2, r4 (middle two swapped)")
    else:
        print("✗ Unknown pattern")

        # Check if it's reversed
        if (r1, r2, r3, r4) == (True, False, True, False):
            print("  -> Pattern is reversed: r4, r3, r2, r1")


def test_different_gates() -> None:
    """Test with different gates to see pattern."""

    @guppy
    def gate_test() -> tuple[bool, bool, bool, bool]:
        # Use different gates for clear pattern
        q1 = qubit()  # |0⟩
        r1 = measure(q1)  # False

        q2 = qubit()
        x(q2)  # X|0⟩ = |1⟩
        r2 = measure(q2)  # True

        q3 = qubit()
        y(q3)  # Y|0⟩ = i|1⟩
        r3 = measure(q3)  # True

        q4 = qubit()
        z(q4)  # Z|0⟩ = |0⟩
        r4 = measure(q4)  # False

        return r1, r2, r3, r4

    results = sim(gate_test).qubits(4).quantum(state_vector()).run(1)
    # Decode integer-encoded results
    decoded_results = decode_integer_results(
        results.get("measurements", results.get("measurement_1", [])),
        4,
    )
    val = decoded_results[0]
    r1, r2, r3, r4 = val

    print("\nGate test:")
    print("Expected: (False[|0⟩], True[X], True[Y], False[Z])")
    print(f"Got:      ({r1}, {r2}, {r3}, {r4})")


if __name__ == "__main__":
    test_tuple_order()
    test_different_gates()
