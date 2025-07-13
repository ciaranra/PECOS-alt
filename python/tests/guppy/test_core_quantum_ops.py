#!/usr/bin/env python3
"""Core quantum operations tests - simplified version."""

import sys
from pathlib import Path
import pytest

sys.path.append("python/quantum-pecos/src")

try:
    from guppylang import guppy
    from guppylang.std.quantum import (
        qubit, measure, discard, reset,
        h, x, y, z, s, sdg, t, tdg,
        cx, cy, cz, ch,
        rx, ry, rz, crz,
        toffoli
    )
    from guppylang.std.angles import angle, pi
    from guppylang.std.builtins import owned
    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False

try:
    from pecos.frontends import guppy_sim
    PECOS_AVAILABLE = True
except ImportError:
    PECOS_AVAILABLE = False


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not PECOS_AVAILABLE, reason="PECOS not available")
class TestSingleQubitGates:
    """Test individual single-qubit gates."""
    
    def test_x_gate(self):
        """Test Pauli-X gate."""
        @guppy
        def x_test() -> bool:
            q = qubit()
            x(q)
            return measure(q)
        
        results = guppy_sim(x_test, max_qubits=5).run(100)
        assert all(r == 1 for r in results["_result"])
    
    def test_y_gate(self):
        """Test Pauli-Y gate."""
        @guppy
        def y_test() -> bool:
            q = qubit()
            y(q)
            return measure(q)
        
        results = guppy_sim(y_test, max_qubits=10).run(100)
        assert all(r == 1 for r in results["_result"])
    
    def test_z_gate(self):
        """Test Pauli-Z gate."""
        @guppy
        def z_test() -> bool:
            q = qubit()
            z(q)
            return measure(q)
        
        results = guppy_sim(z_test, max_qubits=10).run(100)
        assert all(r == 0 for r in results["_result"])
    
    def test_h_gate(self):
        """Test Hadamard gate."""
        @guppy
        def h_test() -> bool:
            q = qubit()
            h(q)
            return measure(q)
        
        results = guppy_sim(h_test, max_qubits=10).seed(42).run(100)
        # Should see both 0 and 1
        zeros = sum(1 for r in results["_result"] if r == 0)
        ones = sum(1 for r in results["_result"] if r == 1)
        assert zeros > 20 and ones > 20
    
    def test_s_gate(self):
        """Test S gate."""
        @guppy
        def s_test() -> bool:
            q = qubit()
            x(q)  # |1⟩
            s(q)  # Phase gate
            return measure(q)
        
        results = guppy_sim(s_test, max_qubits=10).run(100)
        # S gate doesn't change computational basis
        assert all(r == 1 for r in results["_result"])
    
    def test_t_gate(self):
        """Test T gate."""
        @guppy
        def t_test() -> bool:
            q = qubit()
            x(q)  # |1⟩
            t(q)  # π/8 gate
            return measure(q)
        
        results = guppy_sim(t_test, max_qubits=10).run(100)
        # T gate doesn't change computational basis
        assert all(r == 1 for r in results["_result"])


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not PECOS_AVAILABLE, reason="PECOS not available")
class TestTwoQubitGates:
    """Test two-qubit gates."""
    
    def test_cx_gate(self):
        """Test CNOT gate."""
        @guppy
        def cx_test() -> tuple[bool, bool]:
            q1 = qubit()
            q2 = qubit()
            x(q1)  # Control = |1⟩
            cx(q1, q2)  # Target flips
            return measure(q1), measure(q2)
        
        results = guppy_sim(cx_test, max_qubits=10).run(100)
        # Should get (True, True) for both qubits
        assert all(r == (True, True) for r in results["_result"])
    
    def test_cz_gate(self):
        """Test CZ gate."""
        @guppy
        def cz_test() -> tuple[bool, bool]:
            q1 = qubit()
            q2 = qubit()
            x(q1)
            x(q2)
            cz(q1, q2)  # Phase when both |1⟩
            return measure(q1), measure(q2)
        
        results = guppy_sim(cz_test, max_qubits=10).run(100)
        # CZ doesn't change computational basis
        # Both qubits remain |1⟩
        assert all(r == (True, True) for r in results["_result"])
    
    def test_cy_gate(self):
        """Test CY gate."""
        @guppy
        def cy_test() -> tuple[bool, bool]:
            q1 = qubit()
            q2 = qubit()
            x(q1)  # Control = |1⟩
            cy(q1, q2)  # Apply Y to target
            return measure(q1), measure(q2)
        
        results = guppy_sim(cy_test, max_qubits=10).run(100)
        # CY with control=1 applies Y to target
        # Y|0⟩ = i|1⟩, so both measure as |1⟩
        assert all(r == (True, True) for r in results["_result"])


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not PECOS_AVAILABLE, reason="PECOS not available")
class TestQuantumStateManagement:
    """Test state management operations."""
    
    def test_reset(self):
        """Test reset operation."""
        @guppy
        def reset_test() -> bool:
            q = qubit()
            x(q)
            reset(q)
            return measure(q)
        
        results = guppy_sim(reset_test, max_qubits=10).run(100)
        # Reset should give |0⟩
        assert all(r == 0 for r in results["_result"])
    
    def test_discard(self):
        """Test discard operation."""
        @guppy
        def discard_test() -> bool:
            q1 = qubit()
            h(q1)
            discard(q1)
            # Allocate new qubit
            q2 = qubit()
            x(q2)
            return measure(q2)
        
        results = guppy_sim(discard_test, max_qubits=10).run(100)
        assert all(r == 1 for r in results["_result"])


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not PECOS_AVAILABLE, reason="PECOS not available")
class TestQuantumCircuits:
    """Test quantum circuit patterns."""
    
    def test_bell_state(self):
        """Test Bell state creation."""
        @guppy
        def bell_test() -> tuple[bool, bool]:
            q1 = qubit()
            q2 = qubit()
            h(q1)
            cx(q1, q2)
            return measure(q1), measure(q2)
        
        results = guppy_sim(bell_test, max_qubits=10).seed(42).run(100)
        # Should only see (False, False) and (True, True)
        for r in results["_result"]:
            assert r == (False, False) or r == (True, True)
    
    def test_ghz_state(self):
        """Test 3-qubit GHZ state."""
        @guppy
        def ghz_test() -> tuple[bool, bool, bool]:
            q1 = qubit()
            q2 = qubit()
            q3 = qubit()
            h(q1)
            cx(q1, q2)
            cx(q2, q3)
            return measure(q1), measure(q2), measure(q3)
        
        results = guppy_sim(ghz_test, max_qubits=10).seed(42).run(100)
        # Should only see (False, False, False) and (True, True, True)
        for r in results["_result"]:
            assert r == (False, False, False) or r == (True, True, True)


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not PECOS_AVAILABLE, reason="PECOS not available")
class TestRotationGates:
    """Test rotation gates."""
    
    def test_rx_gate(self):
        """Test Rx rotation."""
        @guppy
        def rx_test() -> bool:
            q = qubit()
            rx(q, pi)  # Rx(π) = X up to phase
            return measure(q)
        
        results = guppy_sim(rx_test, max_qubits=10).run(100)
        assert all(r == 1 for r in results["_result"])
    
    def test_ry_gate(self):
        """Test Ry rotation."""
        @guppy
        def ry_test() -> bool:
            q = qubit()
            ry(q, pi)  # Ry(π) flips qubit
            return measure(q)
        
        results = guppy_sim(ry_test, max_qubits=10).run(100)
        assert all(r == 1 for r in results["_result"])
    
    def test_rz_gate(self):
        """Test Rz rotation."""
        @guppy
        def rz_test() -> bool:
            q = qubit()
            rz(q, pi)  # Rz on |0⟩
            return measure(q)
        
        results = guppy_sim(rz_test, max_qubits=10).run(100)
        # Rz doesn't change |0⟩ measurement
        assert all(r == 0 for r in results["_result"])


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not PECOS_AVAILABLE, reason="PECOS not available")
class TestControlFlow:
    """Test control flow with quantum operations."""
    
    @pytest.mark.skip(reason="HUGR doesn't support boolean constants in control flow yet")
    def test_conditional_ops(self):
        """Test conditional quantum operations."""
        @guppy
        def conditional_test(flag: bool) -> bool:
            q = qubit()
            if flag:
                x(q)
            return measure(q)
        
        # Test with True
        @guppy
        def test_true() -> bool:
            return conditional_test(True)
        
        # Test with False
        @guppy
        def test_false() -> bool:
            return conditional_test(False)
        
        results_true = guppy_sim(test_true, max_qubits=10).run(100)
        results_false = guppy_sim(test_false, max_qubits=10).run(100)
        
        assert all(r == 1 for r in results_true["_result"])
        assert all(r == 0 for r in results_false["_result"])
    
    def test_loop_with_quantum(self):
        """Test loop with quantum operations."""
        @guppy
        def loop_test() -> int:
            count = 0
            for i in range(3):
                q = qubit()
                h(q)
                if measure(q):
                    count += 1
            return count
        
        results = guppy_sim(loop_test, max_qubits=10).seed(42).run(100)
        # Should see values 0-3
        values = set(results["_result"])
        assert len(values) >= 2  # At least some variation


if __name__ == "__main__":
    print("Running core quantum operations tests...")
    pytest.main([__file__, "-v"])