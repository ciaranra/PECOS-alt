"""Debug test for tuple measurement returns."""

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


def test_single_measurements() -> None:
    """Test single measurements work correctly."""

    @guppy
    def single_y() -> bool:
        q = qubit()
        y(q)  # Y on |0⟩ gives |1⟩
        return measure(q)

    results = sim(single_y).qubits(1).quantum(state_vector()).run(10)
    for val in results.get("measurements", results.get("measurement_1", [])):
        assert val, f"Y on |0⟩ should give True, got {val}"


def test_simple_tuple() -> None:
    """Test simplest tuple return."""

    @guppy
    def simple_tuple() -> tuple[bool, bool]:
        # First qubit: X on |0⟩ gives |1⟩
        q1 = qubit()
        x(q1)
        r1 = measure(q1)

        # Second qubit: |0⟩ gives |0⟩
        q2 = qubit()
        r2 = measure(q2)

        return r1, r2

    results = sim(simple_tuple).qubits(2).quantum(state_vector()).run(10)
    # Decode integer-encoded results
    decoded_results = decode_integer_results(
        results.get("measurements", results.get("measurement_1", [])),
        2,
    )
    for i, val in enumerate(decoded_results):
        r1, r2 = val
        print(f"Shot {i}: r1={r1} (X|0⟩, expect True), r2={r2} (|0⟩, expect False)")
        # Check if all values are the same
        if i == 0:
            print(f"  => Both values are {r1}. They should be different!")
        assert r1, f"X on |0⟩ should give True, got {r1}"
        assert not r2, f"|0⟩ should give False, got {r2}"


def test_direct_tuple_return() -> None:
    """Test direct tuple return without intermediate variables."""

    @guppy
    def direct_tuple() -> tuple[bool, bool]:
        q1 = qubit()
        x(q1)

        q2 = qubit()

        return measure(q1), measure(q2)

    results = sim(direct_tuple).qubits(2).quantum(state_vector()).run(10)
    # Decode integer-encoded results
    decoded_results = decode_integer_results(
        results.get("measurements", results.get("measurement_1", [])),
        2,
    )
    for i, val in enumerate(decoded_results):
        r1, r2 = val
        print(
            f"Shot {i}: Direct return r1={r1} (X|0⟩, expect True), r2={r2} (|0⟩, expect False)",
        )
        assert r1, f"X on |0⟩ should give True, got {r1}"
        assert not r2, f"|0⟩ should give False, got {r2}"


def test_y_gate_tuple() -> None:
    """Test Y gate specifically in tuple."""

    @guppy
    def y_tuple() -> tuple[bool, bool]:
        q1 = qubit()
        y(q1)  # Y on |0⟩ gives |1⟩
        r1 = measure(q1)

        q2 = qubit()
        z(q2)  # Z on |0⟩ gives |0⟩
        r2 = measure(q2)

        return r1, r2

    results = sim(y_tuple).qubits(2).quantum(state_vector()).run(10)
    # Decode integer-encoded results
    decoded_results = decode_integer_results(
        results.get("measurements", results.get("measurement_1", [])),
        2,
    )
    for i, val in enumerate(decoded_results):
        r1, r2 = val
        print(
            f"Shot {i}: Y gate r1={r1} (Y|0⟩, expect True), r2={r2} (Z|0⟩, expect False)",
        )
        assert r1, f"Y on |0⟩ should give True, got {r1}"
        assert not r2, f"Z on |0⟩ should give False, got {r2}"


if __name__ == "__main__":
    print("Testing single measurements...")
    test_single_measurements()
    print("✓ Single measurements work correctly\n")

    print("Testing simple tuple...")
    test_simple_tuple()
    print("✓ Simple tuple works correctly\n")

    print("Testing direct tuple return...")
    test_direct_tuple_return()
    print("✓ Direct tuple return works correctly\n")

    print("Testing Y gate in tuple...")
    test_y_gate_tuple()
    print("✓ Y gate tuple works correctly")
