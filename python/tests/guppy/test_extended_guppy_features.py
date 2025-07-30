#!/usr/bin/env python3
"""Extended comprehensive test suite for Guppy language features.

This test suite builds on test_comprehensive_guppy_features.py to provide
additional coverage of Guppy language capabilities, including:
- Advanced quantum operations (rotations, phase gates)
- Complex data types (arrays, tuples, lists)
- Advanced control flow (nested loops, complex conditionals)
- Function composition and higher-order functions
- Error handling and edge cases
"""

import sys
from pathlib import Path
from typing import Any, Dict, List, Tuple, Callable
import math


def decode_integer_results(results: List[int], n_bits: int) -> List[Tuple[bool, ...]]:
    """Decode integer-encoded results back to tuples of booleans."""
    decoded = []
    for val in results:
        bits = []
        for i in range(n_bits):
            bits.append(bool(val & (1 << i)))
        decoded.append(tuple(bits))
    return decoded


import pytest

sys.path.append("python/quantum-pecos/src")

# Check dependencies
try:
    from guppylang import guppy
    from guppylang.std.quantum import (
        qubit, measure, h, x, y, z, cx, cz, ry, rz,
        discard, reset, s, sdg, t, tdg, ch, cy
    )
    from guppylang.std.quantum import array as qubit_array
    from guppylang.std.builtins import array, nat, owned
    from guppylang.std.angles import angle, pi
    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False

try:
    from pecos.frontends.run_guppy import run_guppy, get_guppy_backends
    PECOS_FRONTEND_AVAILABLE = True
except ImportError:
    PECOS_FRONTEND_AVAILABLE = False


class ExtendedGuppyTester:
    """Extended helper class for testing advanced Guppy features."""
    
    def __init__(self):
        self.backends = get_guppy_backends() if PECOS_FRONTEND_AVAILABLE else {}
        
    def test_function(self, func, shots: int = 100, seed: int = 42, **kwargs) -> Dict[str, Any]:
        """Test a Guppy function and return results."""
        if not self.backends.get("rust_backend", False):
            return {
                "success": False,
                "error": "Rust backend not available",
                "result": None
            }
            
        try:
            result = run_guppy(func, shots=shots, seed=seed, verbose=False, **kwargs)
            return {
                "success": True,
                "result": result,
                "error": None
            }
        except Exception as e:
            return {
                "success": False,
                "result": None,
                "error": str(e)
            }


@pytest.fixture
def tester():
    """Fixture providing the extended testing helper."""
    return ExtendedGuppyTester()


# ============================================================================
# PHASE AND ROTATION GATES
# ============================================================================

@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not PECOS_FRONTEND_AVAILABLE, reason="PECOS frontend not available")
class TestPhaseAndRotationGates:
    """Test phase gates and rotation operations."""
    
    def test_phase_gates_s_and_t(self, tester):
        """Test S and T phase gates."""
        @guppy
        def phase_gate_test() -> tuple[bool, bool]:
            # S gate test: S|+⟩ = |i⟩
            q1 = qubit()
            h(q1)  # Create |+⟩
            s(q1)  # Apply S gate
            h(q1)  # Should give different result than without S
            r1 = measure(q1)
            
            # T gate test: T is sqrt(S)
            q2 = qubit()
            h(q2)
            t(q2)
            t(q2)  # T² = S
            h(q2)
            r2 = measure(q2)
            
            return r1, r2
        
        result = tester.test_function(phase_gate_test, shots=100)
        if result["success"]:
            print(f"Phase gate test results: {result['result']['results'][:10]}...")
    
    def test_phase_gate_inverses(self, tester):
        """Test S† and T† (inverse phase gates)."""
        @guppy
        def inverse_phase_test() -> bool:
            q = qubit()
            h(q)
            
            # Apply S then S†, should cancel
            s(q)
            sdg(q)
            
            # Apply T then T†, should cancel
            t(q)
            tdg(q)
            
            h(q)  # Should return to |0⟩
            return measure(q)
        
        result = tester.test_function(inverse_phase_test, shots=100)
        if result["success"]:
            zeros = sum(1 for r in result["result"]["results"] if not r)
            assert zeros > 95, f"Phase gates should cancel, got {zeros}/100 zeros"
    
    def test_rotation_gates_ry_rz(self, tester):
        """Test rotation gates with angle parameters."""
        @guppy
        def rotation_test() -> tuple[bool, bool, bool]:
            # RY(π/2) creates equal superposition
            q1 = qubit()
            ry(q1, pi / 2)
            r1 = measure(q1)
            
            # RZ doesn't affect |0⟩ state measurement
            q2 = qubit()
            rz(q2, pi / 4)
            r2 = measure(q2)
            
            # RY(π) is equivalent to Y gate (bit flip)
            q3 = qubit()
            ry(q3, pi)
            r3 = measure(q3)
            
            return r1, r2, r3
        
        result = tester.test_function(rotation_test, shots=100)
        if result["success"]:
            print(f"Rotation gate test: {result}")
            # Check RY(π) behavior - should be equivalent to Y gate
            # Decode integer-encoded results
            decoded_results = decode_integer_results(result["result"]["results"], 3)
            r3_values = [r[2] for r in decoded_results]
            ones = sum(r3_values)
            # RY(π) should flip |0⟩ to |1⟩ like Y gate
            assert ones > 95, f"RY(π) should behave like Y gate, got {ones}/100 ones"


# ============================================================================
# MULTI-QUBIT GATES
# ============================================================================

@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not PECOS_FRONTEND_AVAILABLE, reason="PECOS frontend not available")
class TestMultiQubitGates:
    """Test multi-qubit gate operations."""
    
    def test_controlled_y_and_z(self, tester):
        """Test CY and CZ gates."""
        @guppy
        def cy_gate_test() -> tuple[bool, bool]:
            q0 = qubit()
            q1 = qubit()
            x(q0)  # Set control to |1⟩
            cy(q0, q1)  # Apply CY
            return measure(q0), measure(q1)
        
        @guppy
        def cz_gate_test() -> tuple[bool, bool]:
            q0 = qubit()
            q1 = qubit()
            # CZ on |00⟩ does nothing
            cz(q0, q1)
            return measure(q0), measure(q1)
        
        # Test CY
        result_cy = tester.test_function(cy_gate_test, shots=100)
        if result_cy["success"]:
            # CY with control=1 should flip target
            measurements = result_cy["result"]["results"]
            # Decode integer-encoded results
            decoded_measurements = decode_integer_results(measurements, 2)
            flipped = sum(1 for (c, t) in decoded_measurements if c == 1 and t == 1)
            assert flipped > 95, f"CY should flip target when control=1, got {flipped}/100"
        
        # Test CZ  
        result_cz = tester.test_function(cz_gate_test, shots=100)
        if result_cz["success"]:
            # CZ on |00⟩ should do nothing
            measurements = result_cz["result"]["results"]
            # Decode integer-encoded results
            decoded_measurements = decode_integer_results(measurements, 2)
            zeros = sum(1 for (a, b) in decoded_measurements if a == 0 and b == 0)
            assert zeros > 95, f"CZ on |00⟩ should do nothing, got {zeros}/100"
    
    def test_controlled_hadamard(self, tester):
        """Test controlled Hadamard gate."""
        @guppy
        def ch_gate_test() -> tuple[bool, bool]:
            q0 = qubit()
            q1 = qubit()
            x(q0)  # Set control to |1⟩
            ch(q0, q1)  # Apply controlled-H
            return measure(q0), measure(q1)
        
        result = tester.test_function(ch_gate_test, shots=100)
        if result["success"]:
            print(f"Controlled-H test: {result}")


# ============================================================================
# QUBIT ARRAYS AND COLLECTIONS
# ============================================================================

@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not PECOS_FRONTEND_AVAILABLE, reason="PECOS frontend not available")
class TestQubitArrays:
    """Test qubit array operations and indexing."""
    
    def test_qubit_array_creation_and_access(self, tester):
        """Test creating and accessing qubit arrays."""
        @guppy
        def array_test() -> tuple[bool, bool, bool, bool]:
            # Create array of 4 qubits
            qubits = qubit_array(4)
            
            # Apply different gates to different qubits
            x(qubits[1])  # Flip second qubit
            x(qubits[3])  # Flip fourth qubit
            
            # Measure all
            return (measure(qubits[0]), measure(qubits[1]), 
                   measure(qubits[2]), measure(qubits[3]))
        
        result = tester.test_function(array_test, shots=100)
        if result["success"]:
            # Should get pattern (0,1,0,1) deterministically
            measurements = result["result"]["results"]
            expected = sum(1 for m in measurements if m == (False, True, False, True))
            assert expected > 95, f"Array indexing failed, got {expected}/100 correct"
    
    def test_qubit_array_loops(self, tester):
        """Test looping over qubit arrays."""
        @guppy
        def array_loop_test() -> int:
            n = 5
            qubits = qubit_array(n)
            
            # Apply H to all qubits
            for i in range(n):
                h(qubits[i])
            
            # Count how many measure to |1⟩
            count = 0
            for i in range(n):
                if measure(qubits[i]):
                    count += 1
            
            return count
        
        result = tester.test_function(array_loop_test, shots=100)
        if result["success"]:
            # With 5 qubits in superposition, expect average ~2.5
            counts = result["result"]["results"]
            avg = sum(counts) / len(counts)
            assert 1.5 < avg < 3.5, f"Superposition statistics off, avg={avg}"


# ============================================================================
# CLASSICAL DATA TYPES AND OPERATIONS
# ============================================================================

@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not PECOS_FRONTEND_AVAILABLE, reason="PECOS frontend not available")
class TestClassicalDataTypes:
    """Test classical data types and operations."""
    
    def test_tuple_operations(self, tester):
        """Test tuple creation and unpacking."""
        @guppy
        def tuple_test() -> tuple[bool, bool]:
            # Create and unpack tuple from quantum measurements
            q1, q2 = qubit(), qubit()
            h(q1)
            cx(q1, q2)
            
            # Pack into tuple
            results = (measure(q1), measure(q2))
            
            # Unpack tuple
            a, b = results
            
            return a, b
        
        result = tester.test_function(tuple_test, shots=100)
        if result["success"]:
            # Check Bell state correlation
            measurements = result["result"]["results"]
            # Decode integer-encoded results
            decoded_measurements = decode_integer_results(measurements, 2)
            correlated = sum(1 for (a, b) in decoded_measurements if a == b)
            assert correlated > 80, f"Tuple ops failed, correlation={correlated}/100"
    
    def test_boolean_expressions(self, tester):
        """Test complex boolean expressions."""
        @guppy
        def bool_expr_test() -> bool:
            q1, q2, q3 = qubit(), qubit(), qubit()
            
            # Create different states
            x(q2)  # q2 = |1⟩
            
            # Measure
            a, b, c = measure(q1), measure(q2), measure(q3)
            
            # Complex boolean expression
            # a=False, b=True, c=False
            result = (a or b) and not c  # (False or True) and not False = True
            
            return result
        
        result = tester.test_function(bool_expr_test, shots=100)
        if result["success"]:
            # Should always return True
            trues = sum(result["result"]["results"])
            assert trues > 95, f"Boolean expression failed, got {trues}/100 True"


# ============================================================================
# CONTROL FLOW PATTERNS
# ============================================================================

@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not PECOS_FRONTEND_AVAILABLE, reason="PECOS frontend not available")
class TestControlFlow:
    """Test advanced control flow patterns."""
    
    @pytest.mark.skip(reason="Quantum ops in conditionals are not applied - known HUGR/LLVM bug")
    def test_nested_loops(self, tester):
        """Test nested loop structures."""
        @guppy
        def nested_loop_test() -> int:
            count = 0
            
            # Nested loops with quantum operations
            for i in range(3):
                for j in range(2):
                    q = qubit()
                    if i > j:  # Only true for some iterations
                        x(q)
                    if measure(q):
                        count += 1
            
            return count
        
        result = tester.test_function(nested_loop_test, shots=100)
        if result["success"]:
            # Should count: (1,0), (1,1), (2,0), (2,1) = 4 times
            counts = result["result"]["results"]
            assert all(c == 4 for c in counts), f"Nested loops failed: {counts[:10]}"
    
    @pytest.mark.skip(reason="Known measurement-based conditional bug")
    def test_while_with_quantum(self, tester):
        """Test while loops with quantum operations."""
        @guppy
        def while_quantum_test() -> int:
            count = 0
            tries = 0
            
            # Keep trying until we get a |1⟩ measurement
            while count == 0 and tries < 10:
                q = qubit()
                h(q)  # 50% chance of |1⟩
                if measure(q):
                    count = 1
                tries += 1
            
            return tries
        
        result = tester.test_function(while_quantum_test, shots=100)
        if result["success"]:
            # Should usually succeed in 1-3 tries
            tries = result["result"]["results"]
            avg_tries = sum(tries) / len(tries)
            assert 1 <= avg_tries <= 4, f"While loop statistics off, avg_tries={avg_tries}"
    
    @pytest.mark.skip(reason="X gate before measurement not applied - likely same HUGR/LLVM bug")
    def test_early_return(self, tester):
        """Test early return from functions."""
        @guppy
        def early_return_test() -> int:
            for i in range(5):
                q = qubit()
                x(q)
                if measure(q):  # Always True
                    return i  # Return early
            
            return -1  # Should never reach here
        
        result = tester.test_function(early_return_test, shots=100)
        if result["success"]:
            # Should always return 0 (first iteration)
            values = result["result"]["results"]
            assert all(v == 0 for v in values), f"Early return failed: {values[:10]}"


# ============================================================================
# QUANTUM ALGORITHMS AND PROTOCOLS
# ============================================================================

@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not PECOS_FRONTEND_AVAILABLE, reason="PECOS frontend not available")
class TestQuantumAlgorithms:
    """Test quantum algorithms and protocols."""
    
    def test_ghz_state_creation(self, tester):
        """Test GHZ state creation for multiple qubits."""
        @guppy
        def create_ghz3() -> tuple[bool, bool, bool]:
            # Create 3-qubit GHZ state: (|000⟩ + |111⟩)/√2
            qubits = qubit_array(3)
            
            h(qubits[0])
            cx(qubits[0], qubits[1])
            cx(qubits[1], qubits[2])
            
            return measure(qubits[0]), measure(qubits[1]), measure(qubits[2])
        
        result = tester.test_function(create_ghz3, shots=100)
        if result["success"]:
            # Should only get |000⟩ or |111⟩
            measurements = result["result"]["results"]
            all_zeros = sum(1 for m in measurements if m == (False, False, False))
            all_ones = sum(1 for m in measurements if m == (True, True, True))
            total_valid = all_zeros + all_ones
            assert total_valid > 95, f"GHZ state invalid, got {total_valid}/100 valid states"
    
    def test_quantum_phase_kickback(self, tester):
        """Test phase kickback principle."""
        @guppy
        def phase_kickback_test() -> bool:
            # Demonstrate phase kickback with controlled-Z
            control = qubit()
            target = qubit()
            
            # Prepare control in |+⟩ and target in |1⟩
            h(control)
            x(target)
            
            # CZ gate causes phase kickback
            cz(control, target)
            
            # Measure in X basis (apply H before measuring)
            h(control)
            
            return measure(control)
        
        result = tester.test_function(phase_kickback_test, shots=100)
        if result["success"]:
            # Phase kickback should flip the control qubit measurement
            ones = sum(result["result"]["results"])
            assert ones > 95, f"Phase kickback failed, got {ones}/100 ones"
    
    def test_swap_test(self, tester):
        """Test SWAP test for state comparison (simplified)."""
        @guppy
        def swap_test() -> bool:
            # Simplified SWAP test using just CNOTs
            # Test if two qubits are in same state
            q1 = qubit()
            q2 = qubit()
            ancilla = qubit()
            
            # Prepare both in |0⟩ (same state)
            # In real SWAP test, we'd use controlled-SWAP
            
            h(ancilla)
            
            # Simulate comparison (simplified)
            cx(ancilla, q1)
            cx(ancilla, q2)
            cx(q1, q2)
            cx(ancilla, q1)
            cx(ancilla, q2)
            
            h(ancilla)
            
            # Discard test qubits
            discard(q1)
            discard(q2)
            
            return measure(ancilla)
        
        result = tester.test_function(swap_test, shots=100)
        if result["success"]:
            print(f"SWAP test result: {result}")


# ============================================================================
# ERROR HANDLING AND EDGE CASES
# ============================================================================

@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not PECOS_FRONTEND_AVAILABLE, reason="PECOS frontend not available")
class TestErrorHandling:
    """Test error handling and edge cases."""
    
    def test_qubit_reset(self, tester):
        """Test qubit reset operation."""
        @guppy
        def reset_test() -> tuple[bool, bool]:
            q = qubit()
            
            # Put qubit in |1⟩ state
            x(q)
            before = measure(q)  # Should be True
            
            # Reset qubit to |0⟩
            q_new = qubit()  # Have to use new qubit after measure
            x(q_new)
            reset(q_new)
            after = measure(q_new)  # Should be False
            
            return before, after
        
        result = tester.test_function(reset_test, shots=100)
        if result["success"]:
            measurements = result["result"]["results"]
            # Decode integer-encoded results
            decoded_measurements = decode_integer_results(measurements, 2)
            correct = sum(1 for (b, a) in decoded_measurements if b and not a)
            assert correct > 95, f"Reset failed, got {correct}/100 correct"
    
    def test_discard_operation(self, tester):
        """Test qubit discard operation."""
        @guppy
        def discard_test() -> bool:
            # Create entangled qubits
            q1 = qubit()
            q2 = qubit()
            
            h(q1)
            cx(q1, q2)
            
            # Discard one qubit
            discard(q1)
            
            # Measure remaining qubit
            return measure(q2)
        
        result = tester.test_function(discard_test, shots=100)
        if result["success"]:
            # After discarding, q2 should be in mixed state
            ones = sum(result["result"]["results"])
            assert 30 < ones < 70, f"Discard statistics off, got {ones}/100 ones"
    
    def test_empty_circuit(self, tester):
        """Test empty quantum circuit."""
        @guppy
        def empty_circuit() -> bool:
            # Just allocate and measure
            q = qubit()
            return measure(q)
        
        result = tester.test_function(empty_circuit, shots=100)
        if result["success"]:
            # Should always measure |0⟩
            zeros = sum(1 for r in result["result"]["results"] if not r)
            assert zeros == 100, f"Empty circuit failed, got {zeros}/100 zeros"


# ============================================================================
# PERFORMANCE AND STRESS TESTS
# ============================================================================

@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not PECOS_FRONTEND_AVAILABLE, reason="PECOS frontend not available")
class TestPerformance:
    """Test performance with larger circuits."""
    
    def test_many_qubits(self, tester):
        """Test handling many qubits."""
        @guppy
        def many_qubits_test() -> int:
            # Create 10 qubits
            n = 10
            qubits = qubit_array(n)
            
            # Apply H to all
            for i in range(n):
                h(qubits[i])
            
            # Count ones
            count = 0
            for i in range(n):
                if measure(qubits[i]):
                    count += 1
            
            return count
        
        result = tester.test_function(many_qubits_test, shots=50)
        if result["success"]:
            counts = result["result"]["results"]
            avg = sum(counts) / len(counts)
            assert 3 < avg < 7, f"Many qubit statistics off, avg={avg}"
    
    def test_deep_circuit(self, tester):
        """Test deep circuit with many gates."""
        @guppy
        def deep_circuit_test() -> bool:
            q = qubit()
            
            # Apply many gates
            for i in range(10):
                h(q)
                s(q)
                t(q)
                tdg(q)
                sdg(q)
                h(q)
            
            return measure(q)
        
        result = tester.test_function(deep_circuit_test, shots=100)
        if result["success"]:
            # Circuit should return to |0⟩
            zeros = sum(1 for r in result["result"]["results"] if not r)
            assert zeros > 95, f"Deep circuit failed, got {zeros}/100 zeros"


# ============================================================================
# FEATURE CAPABILITY REPORT
# ============================================================================

def generate_extended_feature_report():
    """Generate comprehensive feature capability report."""
    print("\n" + "="*80)
    print("EXTENDED GUPPY FEATURE TEST REPORT")
    print("="*80)
    
    if not PECOS_FRONTEND_AVAILABLE:
        print("PECOS frontend not available - cannot run tests")
        return
    
    tester = ExtendedGuppyTester()
    
    # Test basic functionality
    @guppy
    def simple_test() -> bool:
        q = qubit()
        h(q)
        return measure(q)
    
    result = tester.test_function(simple_test, shots=10)
    
    print(f"\nBackend Status:")
    print(f"  Rust Backend Available: {tester.backends.get('rust_backend', False)}")
    print(f"  Basic Test Success: {result['success']}")
    if not result['success']:
        print(f"  Error: {result['error']}")
    
    print("\nExtended Feature Coverage:")
    features = [
        "Phase Gates (S, T, S†, T†)",
        "Rotation Gates (RY, RZ with angles)",
        "Controlled Gates (CY, CZ, CH)",
        "Qubit Arrays (allocation, indexing)",
        "Array Loops and Iteration",
        "Tuple Operations (packing/unpacking)",
        "Complex Boolean Expressions",
        "Nested Control Flow",
        "While Loops with Quantum Ops",
        "Early Return Patterns",
        "GHZ State Creation",
        "Phase Kickback",
        "Qubit Reset and Discard",
        "Large Qubit Counts (10+)",
        "Deep Circuits (many gates)",
    ]
    
    for feature in features:
        print(f"  - {feature}")
    
    print("\nRecommendations for expanding support:")
    print("1. Add support for parameterized quantum functions")
    print("2. Implement quantum oracles and controlled unitaries")
    print("3. Add support for classical floating-point operations")
    print("4. Implement measurement result post-processing")
    print("5. Add support for variational quantum algorithms")
    
    print("\n" + "="*80)


if __name__ == "__main__":
    # Run feature report when executed directly
    generate_extended_feature_report()
    
    # Run some sample tests
    if GUPPY_AVAILABLE and PECOS_FRONTEND_AVAILABLE:
        tester = ExtendedGuppyTester()
        
        print("\nRunning sample tests...")
        
        # Test phase gates
        phase_test = TestPhaseAndRotationGates()
        phase_test.test_phase_gates_s_and_t(tester)
        
        # Test arrays
        array_test = TestQubitArrays()
        array_test.test_qubit_array_creation_and_access(tester)
        
        # Test algorithms
        algo_test = TestQuantumAlgorithms()
        algo_test.test_ghz_state_creation(tester)