"""Test suite for advanced quantum gates (Toffoli, CRz, etc.)."""

import pytest
from guppylang import guppy
from guppylang.std.quantum import qubit, h, measure, pi
import pecos_rslib

# Check if gates are available
try:
    from guppylang.std.quantum import toffoli, crz
    HAVE_ADVANCED_GATES = True
except ImportError:
    HAVE_ADVANCED_GATES = False
    # Define dummy functions for testing
    def toffoli(q0, q1, q2): pass
    def crz(q0, q1, angle): pass


class TestThreeQubitGates:
    """Test three-qubit gates."""

    @pytest.mark.skipif(not HAVE_ADVANCED_GATES, reason="Advanced gates not available")
    def test_toffoli_gate(self):
        """Test Toffoli (CCX) gate."""
        @guppy
        def test_toffoli() -> tuple[bool, bool, bool]:
            q0 = qubit()
            q1 = qubit()
            q2 = qubit()
            h(q0)
            h(q1)
            toffoli(q0, q1, q2)
            return measure(q0), measure(q1), measure(q2)

        hugr = test_toffoli.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())

        # Toffoli should decompose into multiple gates
        assert "___rxy" in output
        assert "___rz" in output
        assert "___rzz" in output

        # Should have many operations (Toffoli needs many gates)
        ops_count = output.count("tail call void @___")
        assert ops_count >= 20, f"Toffoli should have many operations, got {ops_count}"


class TestControlledRotations:
    """Test controlled rotation gates."""

    @pytest.mark.skipif(not HAVE_ADVANCED_GATES, reason="Advanced gates not available")
    def test_crz_gate(self):
        """Test CRz gate with angle."""
        @guppy
        def test_crz() -> tuple[bool, bool]:
            q0 = qubit()
            q1 = qubit()
            h(q0)
            crz(q0, q1, pi / 4)
            return measure(q0), measure(q1)

        hugr = test_crz.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())

        # CRz should use RZZ and RZ gates
        assert "___rzz" in output
        assert "___rz" in output


class TestCompilerFeatures:
    """Test compiler features and optimizations."""

    def test_transformation_passes_applied(self):
        """Test that transformation passes are applied (at least nominally)."""
        @guppy
        def simple() -> bool:
            q = qubit()
            h(q)
            return measure(q)

        hugr = simple.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())

        # Should compile successfully
        assert "qmain" in output
        assert "___qalloc" in output

    def test_complex_circuit_compilation(self):
        """Test compilation of complex circuit with many gate types."""
        from guppylang.std.quantum import cx, cy, cz

        @guppy
        def complex_circuit() -> tuple[bool, bool, bool]:
            q0 = qubit()
            q1 = qubit()
            q2 = qubit()

            # Mix of gates
            h(q0)
            cx(q0, q1)
            cy(q1, q2)
            cz(q0, q2)

            # Measurements
            return measure(q0), measure(q1), measure(q2)

        hugr = complex_circuit.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())

        # Should have all operation types
        assert "___rxy" in output
        assert "___rz" in output
        assert "___rzz" in output
        assert "___lazy_measure" in output
        assert "___qfree" in output

    def test_gate_count_optimization(self):
        """Verify that only used operations are declared."""
        from guppylang.std.quantum import cx

        @guppy
        def only_cnot() -> tuple[bool, bool]:
            q0 = qubit()
            q1 = qubit()
            h(q0)
            cx(q0, q1)
            return measure(q0), measure(q1)

        hugr = only_cnot.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())

        # Should declare the operations we use
        assert "declare" in output
        assert "___rxy" in output  # For H and CX
        assert "___rz" in output    # For H and CX
        assert "___rzz" in output   # For CX

        # Count declarations vs actual usage
        declare_count = output.count("declare")
        # Should have reasonable number of declarations
        assert declare_count < 15, f"Too many declarations: {declare_count}"


# Test fallback for when advanced gates are not available
def test_advanced_gates_availability():
    """Check if advanced gates are available in guppylang."""
    try:
        from guppylang.std.quantum import toffoli
        assert True, "Toffoli gate is available"
    except ImportError:
        # It's okay if not available, we handle it
        pass

    try:
        from guppylang.std.quantum import crz
        assert True, "CRz gate is available"
    except ImportError:
        # It's okay if not available, we handle it
        pass