#!/usr/bin/env python3
"""Core quantum operations tests - simplified version."""

import sys
from pathlib import Path
import pytest
from typing import List, Tuple


def decode_integer_results(results: List[int], n_bits: int) -> List[Tuple[bool, ...]]:
    """Decode integer-encoded results back to tuples of booleans."""
    decoded = []
    for val in results:
        bits = []
        for i in range(n_bits):
            bits.append(bool(val & (1 << i)))
        decoded.append(tuple(bits))
    return decoded

def get_measurement_tuples(results: dict, n_bits: int) -> List[Tuple[bool, ...]]:
    """Extract measurement tuples from results, handling both formats."""
    # Try new format with individual measurement keys first
    if "measurement_1" in results and n_bits > 1:
        # Combine individual measurement results into tuples
        measurements = []
        measurement_keys = [f"measurement_{i+1}" for i in range(n_bits)]
        
        # Check all required keys exist
        if all(key in results for key in measurement_keys):
            num_shots = len(results["measurement_1"])
            for shot_idx in range(num_shots):
                measurement_tuple = tuple(bool(results[key][shot_idx]) for key in measurement_keys)
                measurements.append(measurement_tuple)
            return measurements
    
    # Fall back to old format with integer encoding
    measurements = results.get("measurements", results.get("measurement_1", results.get("result", [])))
    if n_bits == 1:
        return [(bool(m),) for m in measurements]
    else:
        return decode_integer_results(measurements, n_bits)


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

from pecos.frontends.guppy_api import sim
from pecos_rslib import state_vector


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
class TestSingleQubitGates:
    """Test individual single-qubit gates."""
    
    def test_x_gate(self):
        """Test Pauli-X gate."""
        @guppy
        def x_test() -> bool:
            q = qubit()
            x(q)
            return measure(q)
        
        results = sim(x_test).qubits(5).quantum(state_vector()).run(10)
        measurements = results.get("measurements", results.get("measurement_1", results.get("result", [])))
        assert all(r == 1 for r in measurements)
    
    def test_y_gate(self):
        """Test Pauli-Y gate."""
        @guppy
        def y_test() -> bool:
            q = qubit()
            y(q)
            return measure(q)
        
        results = sim(y_test).qubits(10).quantum(state_vector()).run(10)
        measurements = results.get("measurements", results.get("measurement_1", results.get("result", [])))
        assert all(r == 1 for r in measurements)
    
    def test_z_gate(self):
        """Test Pauli-Z gate."""
        @guppy
        def z_test() -> bool:
            q = qubit()
            z(q)
            return measure(q)
        
        results = sim(z_test).qubits(10).quantum(state_vector()).run(10)
        measurements = results.get("measurements", results.get("measurement_1", results.get("result", [])))
        assert all(r == 0 for r in measurements)
    
    def test_h_gate(self):
        """Test Hadamard gate."""
        @guppy
        def h_test() -> bool:
            q = qubit()
            h(q)
            return measure(q)
        
        results = sim(h_test).qubits(10).quantum(state_vector()).run(10)
        # Should see both 0 and 1
        measurements = results.get("measurements", results.get("measurement_1", results.get("result", [])))
        zeros = sum(1 for r in measurements if r == 0)
        ones = sum(1 for r in measurements if r == 1)
        assert zeros > 0 and ones > 0  # Should see both outcomes
    
    def test_s_gate(self):
        """Test S gate."""
        @guppy
        def s_test() -> bool:
            q = qubit()
            x(q)  # |1⟩
            s(q)  # Phase gate
            return measure(q)
        
        results = sim(s_test).qubits(10).quantum(state_vector()).run(10)
        # S gate doesn't change computational basis
        measurements = results.get("measurements", results.get("measurement_1", results.get("result", [])))
        assert all(r == 1 for r in measurements)
    
    def test_t_gate(self):
        """Test T gate."""
        @guppy
        def t_test() -> bool:
            q = qubit()
            x(q)  # |1⟩
            t(q)  # π/8 gate
            return measure(q)
        
        results = sim(t_test).qubits(10).quantum(state_vector()).run(10)
        # T gate doesn't change computational basis
        measurements = results.get("measurements", results.get("measurement_1", results.get("result", [])))
        assert all(r == 1 for r in measurements)


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
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
        
        results = sim(cx_test).qubits(10).quantum(state_vector()).run(10)
        # Should get (True, True) for both qubits
        decoded_results = get_measurement_tuples(results, 2)
        assert all(r == (True, True) for r in decoded_results)
    
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
        
        results = sim(cz_test).qubits(10).quantum(state_vector()).run(10)
        # CZ doesn't change computational basis, both qubits remain |1⟩
        decoded_results = get_measurement_tuples(results, 2)
        assert all(r == (True, True) for r in decoded_results)
    
    def test_cy_gate(self):
        """Test CY gate."""
        @guppy
        def cy_test() -> tuple[bool, bool]:
            q1 = qubit()
            q2 = qubit()
            x(q1)  # Control = |1⟩
            cy(q1, q2)  # Apply Y to target
            return measure(q1), measure(q2)
        
        results = sim(cy_test).qubits(10).quantum(state_vector()).run(10)
        # CY with control=1 applies Y to target, Y|0⟩ = i|1⟩, so both measure as |1⟩
        decoded_results = get_measurement_tuples(results, 2)
        assert all(r == (True, True) for r in decoded_results)


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
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
        
        results = sim(reset_test).qubits(10).quantum(state_vector()).run(10)
        # Reset should give |0⟩
        measurements = results.get("measurements", results.get("measurement_1", results.get("result", [])))
        assert all(r == 0 for r in measurements)
    
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
        
        results = sim(discard_test).qubits(10).quantum(state_vector()).run(10)
        measurements = results.get("measurements", results.get("measurement_1", results.get("result", [])))
        assert all(r == 1 for r in measurements)


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
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
        
        results = sim(bell_test).qubits(10).quantum(state_vector()).seed(42).run(100)
        # Bell state should be correlated
        decoded = get_measurement_tuples(results, 2)
        for (a, b) in decoded:
            assert a == b  # Bell state is correlated
    
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
        
        results = sim(ghz_test).qubits(10).quantum(state_vector()).seed(42).run(100)
        # GHZ state should be all-correlated
        decoded = get_measurement_tuples(results, 3)
        for (a, b, c) in decoded:
            assert (a == b == c)  # GHZ state is all-correlated


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
class TestRotationGates:
    """Test rotation gates."""
    
    def test_rx_gate(self):
        """Test Rx rotation."""
        @guppy
        def rx_test() -> bool:
            q = qubit()
            rx(q, pi)  # Rx(π) = X up to phase
            return measure(q)
        
        results = sim(rx_test).qubits(10).quantum(state_vector()).run(10)
        measurements = results.get("measurements", results.get("measurement_1", results.get("result", [])))
        assert all(r == 1 for r in measurements)
    
    def test_ry_gate(self):
        """Test Ry rotation."""
        @guppy
        def ry_test() -> bool:
            q = qubit()
            ry(q, pi)  # Ry(π) flips qubit
            return measure(q)
        
        results = sim(ry_test).qubits(10).quantum(state_vector()).run(10)
        measurements = results.get("measurements", results.get("measurement_1", results.get("result", [])))
        assert all(r == 1 for r in measurements)
    
    def test_rz_gate(self):
        """Test Rz rotation."""
        @guppy
        def rz_test() -> bool:
            q = qubit()
            rz(q, pi)  # Rz on |0⟩
            return measure(q)
        
        results = sim(rz_test).qubits(10).quantum(state_vector()).run(10)
        # Rz doesn't change |0⟩ measurement
        measurements = results.get("measurements", results.get("measurement_1", results.get("result", [])))
        assert all(r == 0 for r in measurements)


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
class TestControlFlow:
    """Test control flow with quantum operations."""
    
    def test_conditional_ops(self):
        """Test conditional quantum operations with boolean constants."""
        # Skip this test due to function call compilation issues
        pytest.skip("Function calls with parameters not yet supported in HUGR to LLVM compilation")
    
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
        
        results = sim(loop_test).qubits(10).quantum(state_vector()).seed(42).run(100)
        # Should see values 0-3
        measurements = results.get("measurements", results.get("measurement_1", results.get("result", [])))
        values = set(measurements)
        assert len(values) >= 2  # At least some variation


if __name__ == "__main__":
    print("Running core quantum operations tests...")
    pytest.main([__file__, "-v"])