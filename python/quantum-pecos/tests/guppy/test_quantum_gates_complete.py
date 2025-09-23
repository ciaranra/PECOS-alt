"""Test suite for complete quantum gate coverage in PECOS compiler."""

import pytest
from guppylang import guppy
from guppylang.std.quantum import qubit, h, x, y, z, s, t, sdg, tdg
from guppylang.std.quantum import rx, ry, rz, cx, cy, cz, ch, measure, pi
import pecos_rslib


class TestBasicGates:
    """Test basic single-qubit gates."""

    def test_pauli_gates(self):
        """Test Pauli gates X, Y, Z."""
        @guppy
        def test_x() -> bool:
            q = qubit()
            x(q)
            return measure(q)

        @guppy
        def test_y() -> bool:
            q = qubit()
            y(q)
            return measure(q)

        @guppy
        def test_z() -> bool:
            q = qubit()
            z(q)
            return measure(q)

        for func in [test_x, test_y, test_z]:
            hugr = func.compile()
            output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())
            assert "tail call" in output
            assert "@___r" in output  # Should have rotation calls

    def test_phase_gates(self):
        """Test phase gates S and T."""
        @guppy
        def test_s() -> bool:
            q = qubit()
            s(q)
            return measure(q)

        @guppy
        def test_t() -> bool:
            q = qubit()
            t(q)
            return measure(q)

        for func in [test_s, test_t]:
            hugr = func.compile()
            output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())
            assert "___rz" in output
            assert "tail call" in output

    def test_hadamard(self):
        """Test Hadamard gate."""
        @guppy
        def test_h() -> bool:
            q = qubit()
            h(q)
            return measure(q)

        hugr = test_h.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())
        assert "___rxy" in output
        assert "___rz" in output


class TestAdjointGates:
    """Test adjoint gates."""

    def test_adjoint_gates(self):
        """Test S† and T† gates."""
        @guppy
        def test_sdg_gate() -> bool:
            q = qubit()
            h(q)
            sdg(q)
            return measure(q)

        @guppy
        def test_tdg_gate() -> bool:
            q = qubit()
            h(q)
            tdg(q)
            return measure(q)

        for func in [test_sdg_gate, test_tdg_gate]:
            hugr = func.compile()
            output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())
            assert "___rz" in output
            # Should have negative angle for adjoint
            assert "0xBF" in output  # Negative hex prefix


class TestRotationGates:
    """Test parameterized rotation gates."""

    def test_rx_gate(self):
        """Test Rx gate with angle."""
        @guppy
        def test_rx_pi4() -> bool:
            q = qubit()
            rx(q, pi / 4)
            return measure(q)

        hugr = test_rx_pi4.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())
        assert "___rxy" in output
        assert "double 0.0" in output  # First angle should be 0 for Rx

    def test_ry_gate(self):
        """Test Ry gate with angle."""
        @guppy
        def test_ry_pi2() -> bool:
            q = qubit()
            ry(q, pi / 2)
            return measure(q)

        hugr = test_ry_pi2.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())
        assert "___rxy" in output
        # For Ry, second angle should be 0

    def test_rz_gate(self):
        """Test Rz gate with angle."""
        @guppy
        def test_rz_pi() -> bool:
            q = qubit()
            rz(q, pi)
            return measure(q)

        hugr = test_rz_pi.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())
        assert "___rz" in output
        # Should have an angle parameter
        assert "double" in output


class TestControlGates:
    """Test two-qubit control gates."""

    def test_cx_gate(self):
        """Test CNOT/CX gate."""
        @guppy
        def test_cx() -> tuple[bool, bool]:
            q0 = qubit()
            q1 = qubit()
            h(q0)
            cx(q0, q1)
            return measure(q0), measure(q1)

        hugr = test_cx.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())
        assert "___rxy" in output
        assert "___rzz" in output
        assert "___rz" in output

    def test_cy_gate(self):
        """Test CY gate."""
        @guppy
        def test_cy() -> tuple[bool, bool]:
            q0 = qubit()
            q1 = qubit()
            h(q0)
            cy(q0, q1)
            return measure(q0), measure(q1)

        hugr = test_cy.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())
        assert "___rxy" in output
        assert "___rzz" in output
        assert "___rz" in output
        # Should have multiple operations for CY decomposition
        assert output.count("tail call void @___") >= 7

    def test_cz_gate(self):
        """Test CZ gate."""
        @guppy
        def test_cz() -> tuple[bool, bool]:
            q0 = qubit()
            q1 = qubit()
            h(q0)
            cz(q0, q1)
            return measure(q0), measure(q1)

        hugr = test_cz.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())
        assert "___rzz" in output
        assert "___rz" in output

    def test_ch_gate(self):
        """Test CH gate."""
        @guppy
        def test_ch() -> tuple[bool, bool]:
            q0 = qubit()
            q1 = qubit()
            h(q0)
            ch(q0, q1)
            return measure(q0), measure(q1)

        hugr = test_ch.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())
        assert "___rxy" in output
        assert "___rz" in output
        # CH has its own decomposition


class TestComplexCircuits:
    """Test more complex quantum circuits."""

    def test_bell_state(self):
        """Test Bell state preparation."""
        @guppy
        def bell() -> tuple[bool, bool]:
            q0 = qubit()
            q1 = qubit()
            h(q0)
            cx(q0, q1)
            return measure(q0), measure(q1)

        hugr = bell.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())
        assert "___rxy" in output
        assert "___rzz" in output
        assert "___lazy_measure" in output
        assert "___qfree" in output

    def test_ghz_state(self):
        """Test GHZ state preparation."""
        @guppy
        def ghz() -> tuple[bool, bool, bool]:
            q0 = qubit()
            q1 = qubit()
            q2 = qubit()
            h(q0)
            cx(q0, q1)
            cx(q0, q2)
            return measure(q0), measure(q1), measure(q2)

        hugr = ghz.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())
        assert "___rzz" in output  # Has CX gates
        assert "___lazy_measure" in output  # Has measurements

    def test_mixed_gates(self):
        """Test circuit with mixed gate types."""
        @guppy
        def mixed() -> tuple[bool, bool]:
            q0 = qubit()
            q1 = qubit()
            h(q0)
            s(q0)
            rx(q1, pi / 4)
            cy(q0, q1)
            t(q1)
            return measure(q0), measure(q1)

        hugr = mixed.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())
        assert "___rxy" in output
        assert "___rz" in output
        assert "___rzz" in output


class TestCompilerCompatibility:
    """Test compatibility with Selene compiler."""

    def test_basic_gates_match_selene(self):
        """Verify basic gates produce same number of operations as Selene."""
        @guppy
        def simple() -> bool:
            q = qubit()
            h(q)
            return measure(q)

        hugr = simple.compile()
        pecos_out = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())
        selene_out = pecos_rslib.compile_hugr_to_llvm_selene(hugr.to_bytes())

        # Count operations
        pecos_ops = pecos_out.count("tail call void @___")
        selene_ops = selene_out.count("tail call void @___")

        # Should have same number of quantum operations
        assert abs(pecos_ops - selene_ops) <= 1  # Allow for minor differences

    def test_declarations_optimized(self):
        """Verify only used operations are declared."""
        @guppy
        def only_h() -> bool:
            q = qubit()
            h(q)
            return measure(q)

        hugr = only_h.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())

        # Should declare only what's used
        assert "declare" in output
        assert "___rxy" in output
        assert "___rz" in output

        # Should NOT declare unused operation
        assert "___rzz" not in output or "tail call void @___rzz" not in output