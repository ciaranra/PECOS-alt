"""Test suite for Reset operation."""

import pytest
from guppylang import guppy
from guppylang.std.quantum import qubit, h, x, reset, measure
import pecos_rslib


class TestResetOperation:
    """Test reset operation."""

    def test_reset_basic(self):
        """Test basic reset operation."""
        @guppy
        def test_reset() -> bool:
            q = qubit()
            h(q)  # Put in superposition
            reset(q)  # Reset to |0⟩
            return measure(q)

        hugr = test_reset.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())

        # Should have reset operation
        assert "___reset" in output
        assert "tail call void @___reset" in output

    def test_reset_after_x(self):
        """Test reset after X gate."""
        @guppy
        def test_reset_x() -> bool:
            q = qubit()
            x(q)  # Flip to |1⟩
            reset(q)  # Reset to |0⟩
            return measure(q)

        hugr = test_reset_x.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())

        # Should have both X gate operations and reset
        assert "___rxy" in output  # X gate uses RXY
        assert "___reset" in output

    def test_multiple_resets(self):
        """Test multiple reset operations."""
        @guppy
        def test_multi_reset() -> bool:
            q = qubit()
            h(q)
            reset(q)
            x(q)
            reset(q)
            return measure(q)

        hugr = test_multi_reset.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())

        # Should have two reset calls (plus potentially one from QAlloc)
        reset_calls = output.count("tail call void @___reset")
        assert reset_calls >= 2, f"Expected at least 2 reset calls, got {reset_calls}"

    def test_reset_two_qubits(self):
        """Test reset on two qubits."""
        @guppy
        def test_reset_two() -> tuple[bool, bool]:
            q1 = qubit()
            q2 = qubit()
            h(q1)
            h(q2)
            reset(q1)
            reset(q2)
            return measure(q1), measure(q2)

        hugr = test_reset_two.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())

        # Should have multiple reset calls
        assert "___reset" in output
        # Should have at least 2 reset calls from the Reset operations
        # (plus 2 from QAlloc initialization)
        reset_calls = output.count("tail call void @___reset")
        assert reset_calls >= 4, f"Expected at least 4 reset calls, got {reset_calls}"

    def test_reset_compiler_compatibility(self):
        """Verify reset operation compiles correctly."""
        @guppy
        def simple_reset() -> bool:
            q = qubit()
            reset(q)
            return measure(q)

        hugr = simple_reset.compile()
        pecos_out = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())

        # Should declare and use reset
        assert "declare" in pecos_out
        assert "___reset" in pecos_out
        assert "___lazy_measure" in pecos_out
        assert "___qfree" in pecos_out

        # Test with Selene too
        selene_out = pecos_rslib.compile_hugr_to_llvm_selene(hugr.to_bytes())

        # Both should have reset operations
        assert "___reset" in selene_out

    def test_reset_in_circuit(self):
        """Test reset in a more complex circuit."""
        from guppylang.std.quantum import cx

        @guppy
        def reset_circuit() -> tuple[bool, bool]:
            q1 = qubit()
            q2 = qubit()
            h(q1)
            cx(q1, q2)  # Entangle
            reset(q1)   # Reset control qubit
            # q2 should still be in a mixed state
            return measure(q1), measure(q2)

        hugr = reset_circuit.compile()
        output = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_bytes())

        # Should have all operations
        assert "___rxy" in output  # From H and CX
        assert "___rzz" in output  # From CX
        assert "___reset" in output  # From reset operation