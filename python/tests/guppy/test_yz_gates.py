"""Test Y and Z gates specifically."""

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


def test_y_gate_only() -> None:
    """Test Y gate by itself."""

    @guppy
    def y_only() -> bool:
        q = qubit()
        y(q)
        return measure(q)

    results = sim(y_only).qubits(1).quantum(state_vector()).run(5)
    for i, val in enumerate(
        results.get("measurements", results.get("measurement_1", [])),
    ):
        print(f"Shot {i}: Y|0⟩ = {val} (should be True)")
        assert val


def test_z_gate_only() -> None:
    """Test Z gate by itself."""

    @guppy
    def z_only() -> bool:
        q = qubit()
        z(q)
        return measure(q)

    results = sim(z_only).qubits(1).quantum(state_vector()).run(5)
    for i, val in enumerate(
        results.get("measurements", results.get("measurement_1", [])),
    ):
        print(f"Shot {i}: Z|0⟩ = {val} (should be False)")
        assert not val


def test_y_and_z_tuple() -> None:
    """Test Y and Z gates in a tuple."""

    @guppy
    def yz_tuple() -> tuple[bool, bool]:
        q1 = qubit()
        y(q1)  # Y|0⟩ = i|1⟩
        r1 = measure(q1)

        q2 = qubit()
        z(q2)  # Z|0⟩ = |0⟩
        r2 = measure(q2)

        return r1, r2

    results = sim(yz_tuple).qubits(2).quantum(state_vector()).run(5)
    # Decode integer-encoded results
    decoded_results = decode_integer_results(
        results.get("measurements", results.get("measurement_1", [])),
        2,
    )
    for i, val in enumerate(decoded_results):
        r1, r2 = val
        print(f"Shot {i}: Y|0⟩ = {r1} (should be True), Z|0⟩ = {r2} (should be False)")
        if r1 == r2:
            print(f"  ERROR: Both values are {r1}!")
        assert r1
        assert not r2


def test_xyz_tuple() -> None:
    """Test X, Y, Z gates in a tuple."""

    @guppy
    def xyz_tuple() -> tuple[bool, bool, bool]:
        q1 = qubit()
        x(q1)  # X|0⟩ = |1⟩
        r1 = measure(q1)

        q2 = qubit()
        y(q2)  # Y|0⟩ = i|1⟩
        r2 = measure(q2)

        q3 = qubit()
        z(q3)  # Z|0⟩ = |0⟩
        r3 = measure(q3)

        return r1, r2, r3

    results = sim(xyz_tuple).qubits(3).quantum(state_vector()).run(5)
    # Decode integer-encoded results
    decoded_results = decode_integer_results(
        results.get("measurements", results.get("measurement_1", [])),
        3,
    )
    for i, val in enumerate(decoded_results):
        r1, r2, r3 = val
        print(f"Shot {i}: X|0⟩ = {r1}, Y|0⟩ = {r2}, Z|0⟩ = {r3}")
        print("  Expected: (True, True, False)")
        assert r1
        assert r2
        assert not r3


if __name__ == "__main__":
    print("Testing Y gate only...")
    test_y_gate_only()
    print("✓ Y gate works correctly\n")

    print("Testing Z gate only...")
    test_z_gate_only()
    print("✓ Z gate works correctly\n")

    print("Testing Y and Z in tuple...")
    test_y_and_z_tuple()
    print("✓ Y and Z tuple works correctly\n")

    print("Testing X, Y, Z in tuple...")
    test_xyz_tuple()
    print("✓ X, Y, Z tuple works correctly")
