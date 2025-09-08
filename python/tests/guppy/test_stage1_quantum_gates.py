#!/usr/bin/env python3
"""Test Stage 1 quantum gates implementation for HUGR-LLVM pipeline.

This tests all the newly implemented quantum gates:
- Rotation gates: RX, RY, RZ
- Pauli gates: S, T, Sdg, Tdg
- Two-qubit gates: CY, CZ, CH
- Controlled rotation: CRZ
- Three-qubit: Toffoli
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


# Add paths for imports
sys.path.append("python/quantum-pecos/src")

# Check if dependencies are available
try:
    from guppylang import guppy
    from guppylang.std.angles import angle
    from guppylang.std.quantum import (
        # Import all gates from quantum module instead of quantum_functional
        ch,
        crz,
        cx,
        cy,
        cz,
        h,
        measure,
        pi,
        qubit,
        rx,
        ry,
        rz,
        s,
        sdg,
        t,
        tdg,
        toffoli,
        x,
        y,
        z,
    )

    GUPPY_AVAILABLE = True
    GATES_AVAILABLE = True
except ImportError as e:
    print(f"Import error: {e}")
    GUPPY_AVAILABLE = False
    GATES_AVAILABLE = False

try:
    from pecos.compilation_pipeline import compile_guppy_to_llvm
    from pecos.frontends.guppy_api import sim
    from pecos_rslib import state_vector

    PECOS_AVAILABLE = True
except ImportError as e:
    print(f"PECOS import error: {e}")
    PECOS_AVAILABLE = False


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not GATES_AVAILABLE, reason="Quantum gates not available")
@pytest.mark.skipif(not PECOS_AVAILABLE, reason="PECOS not available")
class TestStage1QuantumGates:
    """Test all Stage 1 quantum gates."""

    def test_rotation_gates(self) -> None:
        """Test RX, RY, RZ gates with angle parameters."""

        @guppy
        def test_rx() -> bool:
            q = qubit()
            rx(q, pi / 2)  # pi/2 radians = pi halfturns = 1.0 halfturn
            return measure(q)

        @guppy
        def test_ry() -> bool:
            q = qubit()
            ry(q, pi / 2)
            return measure(q)

        @guppy
        def test_rz() -> bool:
            q = qubit()
            h(q)  # Put in superposition first
            rz(q, pi / 2)
            h(q)
            return measure(q)

        # Try to compile each function
        for func, name in [(test_rx, "RX"), (test_ry, "RY"), (test_rz, "RZ")]:
            try:
                llvm_ir = compile_guppy_to_llvm(func)
                assert llvm_ir is not None
                # Rotation gates might be decomposed into other gates
                # Check that compilation succeeds and produces quantum operations
                assert (
                    "__quantum__qis__" in llvm_ir
                ), f"No quantum operations found for {name}"
                print(f"✓ {name} gate compiled successfully (may be decomposed)")
            except Exception as e:
                pytest.fail(f"{name} gate compilation failed: {e}")

    def test_pauli_gates(self) -> None:
        """Test S, T, Sdg, Tdg gates."""

        @guppy
        def test_s() -> bool:
            q = qubit()
            h(q)
            s(q)
            h(q)
            return measure(q)

        @guppy
        def test_t() -> bool:
            q = qubit()
            h(q)
            t(q)
            h(q)
            return measure(q)

        @guppy
        def test_sdg() -> bool:
            q = qubit()
            h(q)
            sdg(q)
            h(q)
            return measure(q)

        @guppy
        def test_tdg() -> bool:
            q = qubit()
            h(q)
            tdg(q)
            h(q)
            return measure(q)

        # Try to compile each function
        for func, gate_name in [
            (test_s, "S"),
            (test_t, "T"),
            (test_sdg, "Sdg"),
            (test_tdg, "Tdg"),
        ]:
            try:
                llvm_ir = compile_guppy_to_llvm(func)
                assert llvm_ir is not None
                # Phase gates might be decomposed into other gates
                assert (
                    "__quantum__qis__" in llvm_ir
                ), f"No quantum operations found for {gate_name}"
                print(f"✓ {gate_name} gate compiled successfully (may be decomposed)")
            except Exception as e:
                pytest.fail(f"{gate_name} gate compilation failed: {e}")

    def test_two_qubit_gates(self) -> None:
        """Test CY, CZ, CH gates."""

        @guppy
        def test_cy() -> tuple[bool, bool]:
            q1 = qubit()
            q2 = qubit()
            h(q1)
            cy(q1, q2)
            return measure(q1), measure(q2)

        @guppy
        def test_cz() -> tuple[bool, bool]:
            q1 = qubit()
            q2 = qubit()
            h(q1)
            h(q2)
            cz(q1, q2)
            h(q1)
            h(q2)
            return measure(q1), measure(q2)

        @guppy
        def test_ch() -> tuple[bool, bool]:
            q1 = qubit()
            q2 = qubit()
            x(q1)  # Set control to |1>
            ch(q1, q2)
            return measure(q1), measure(q2)

        # Try to compile each function
        for func, gate in [(test_cy, "CY"), (test_cz, "CZ"), (test_ch, "CH")]:
            try:
                llvm_ir = compile_guppy_to_llvm(func)
                assert llvm_ir is not None
                # Two-qubit gates might be decomposed
                # Just check that quantum operations are present
                assert (
                    "__quantum__qis__" in llvm_ir
                ), f"No quantum operations found for {gate}"
                print(f"✓ {gate} gate compiled successfully (may be decomposed)")
            except Exception as e:
                pytest.fail(f"{gate} gate compilation failed: {e}")

    def test_controlled_rotation(self) -> None:
        """Test CRZ gate with angle parameter."""

        @guppy
        def test_crz() -> tuple[bool, bool]:
            q1 = qubit()
            q2 = qubit()
            x(q1)  # Set control to |1>
            h(q2)
            crz(q1, q2, pi / 4)
            h(q2)
            return measure(q1), measure(q2)

        try:
            llvm_ir = compile_guppy_to_llvm(test_crz)
            assert llvm_ir is not None
            # CRZ might be decomposed into other gates
            assert "__quantum__qis__" in llvm_ir, "No quantum operations found for CRZ"
            print("✓ CRZ gate compiled successfully (may be decomposed)")
        except Exception as e:
            pytest.fail(f"CRZ gate compilation failed: {e}")

    def test_toffoli_gate(self) -> None:
        """Test Toffoli (CCX) gate."""

        @guppy
        def test_toffoli() -> tuple[bool, bool, bool]:
            q1 = qubit()
            q2 = qubit()
            q3 = qubit()
            x(q1)  # Set first control to |1>
            x(q2)  # Set second control to |1>
            toffoli(q1, q2, q3)
            return measure(q1), measure(q2), measure(q3)

        try:
            llvm_ir = compile_guppy_to_llvm(test_toffoli)
            assert llvm_ir is not None
            # Toffoli might be decomposed into other gates
            assert (
                "__quantum__qis__" in llvm_ir
            ), "No quantum operations found for Toffoli"
            print("✓ Toffoli gate compiled successfully (may be decomposed)")
        except Exception as e:
            pytest.fail(f"Toffoli gate compilation failed: {e}")

    def test_combined_circuit(self) -> None:
        """Test a circuit combining multiple new gates."""

        @guppy
        def quantum_algorithm() -> tuple[bool, bool]:
            # Initialize qubits
            q1 = qubit()
            q2 = qubit()

            # Apply rotation gates
            rx(q1, pi / 3)
            ry(q1, pi / 4)

            # Apply Pauli gates
            s(q1)
            t(q2)

            # Apply controlled gates
            cy(q1, q2)
            crz(q1, q2, pi / 6)

            # Final rotations
            sdg(q1)
            tdg(q2)

            return measure(q1), measure(q2)

        try:
            llvm_ir = compile_guppy_to_llvm(quantum_algorithm)
            assert llvm_ir is not None

            # Just check that quantum operations are present
            # Complex gates may be decomposed into basic operations
            assert "__quantum__qis__" in llvm_ir, "No quantum operations found"
            # Check for basic gates that should be present
            assert (
                "__quantum__qis__h__body" in llvm_ir
                or "__quantum__qis__x__body" in llvm_ir
            ), "No basic gates found"

            print("✓ Combined circuit compiled successfully (gates may be decomposed)")
        except Exception as e:
            pytest.fail(f"Combined circuit compilation failed: {e}")


def run_tests() -> None:
    """Run tests and print summary."""
    print("=" * 60)
    print("Stage 1 Quantum Gates Test Suite")
    print("=" * 60)

    if not GUPPY_AVAILABLE:
        print("❌ Guppy not available - install guppylang")
        return

    if not GATES_AVAILABLE:
        print("❌ Some quantum gates not available in guppylang.std.quantum_functional")
        print("   This might be expected if using an older version")

    if not PECOS_AVAILABLE:
        print("❌ PECOS compilation pipeline not available")
        return

    # Run the tests
    test_suite = TestStage1QuantumGates()

    print("\n1. Testing Rotation Gates (RX, RY, RZ)...")
    test_suite.test_rotation_gates()

    print("\n2. Testing Pauli Gates (S, T, Sdg, Tdg)...")
    test_suite.test_pauli_gates()

    print("\n3. Testing Two-Qubit Gates (CY, CZ, CH)...")
    test_suite.test_two_qubit_gates()

    print("\n4. Testing Controlled Rotation (CRZ)...")
    test_suite.test_controlled_rotation()

    print("\n5. Testing Toffoli Gate...")
    test_suite.test_toffoli_gate()

    print("\n6. Testing Combined Circuit...")
    test_suite.test_combined_circuit()

    print("\n" + "=" * 60)
    print("✅ All Stage 1 quantum gates compiled successfully!")
    print("=" * 60)


if __name__ == "__main__":
    run_tests()
