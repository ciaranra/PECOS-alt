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
from pathlib import Path
import pytest

# Add paths for imports
sys.path.append("python/quantum-pecos/src")

# Check if dependencies are available
try:
    from guppylang import guppy
    from guppylang.std.quantum import qubit, measure
    from guppylang.std.angles import angle, pi
    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False

try:
    # Import quantum gates - check if they're available
    from guppylang.std.quantum_functional import (
        h, x, y, z, cx, cy, cz, ch,
        rx, ry, rz, crz,
        s, t, sdg, tdg,
        toffoli
    )
    GATES_AVAILABLE = True
except ImportError:
    GATES_AVAILABLE = False

try:
    from pecos.frontends.run_guppy import run_guppy, get_guppy_backends
    from pecos.compilation_pipeline import compile_guppy_to_llvm
    PECOS_AVAILABLE = True
except ImportError:
    PECOS_AVAILABLE = False


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not GATES_AVAILABLE, reason="Quantum gates not available")
@pytest.mark.skipif(not PECOS_AVAILABLE, reason="PECOS not available")
class TestStage1QuantumGates:
    """Test all Stage 1 quantum gates."""
    
    def test_rotation_gates(self):
        """Test RX, RY, RZ gates with angle parameters."""
        
        @guppy
        def test_rx() -> bool:
            q = qubit()
            q = rx(q, pi/2)  # pi/2 radians = pi/2 halfturns = 0.5 halfturns
            return measure(q)
        
        @guppy
        def test_ry() -> bool:
            q = qubit()
            q = ry(q, pi/2)
            return measure(q)
        
        @guppy
        def test_rz() -> bool:
            q = qubit()
            q = h(q)  # Put in superposition first
            q = rz(q, pi/2)
            q = h(q)
            return measure(q)
        
        # Try to compile each function
        for func, name in [(test_rx, "RX"), (test_ry, "RY"), (test_rz, "RZ")]:
            try:
                llvm_ir = compile_guppy_to_llvm(func)
                assert llvm_ir is not None
                assert f"__quantum__qis__{name.lower()}__body" in llvm_ir
                print(f"✓ {name} gate compiled successfully")
            except Exception as e:
                pytest.fail(f"{name} gate compilation failed: {e}")
    
    def test_pauli_gates(self):
        """Test S, T, Sdg, Tdg gates."""
        
        @guppy
        def test_s() -> bool:
            q = qubit()
            q = h(q)
            q = s(q)
            q = h(q)
            return measure(q)
        
        @guppy
        def test_t() -> bool:
            q = qubit()
            q = h(q)
            q = t(q)
            q = h(q)
            return measure(q)
        
        @guppy
        def test_sdg() -> bool:
            q = qubit()
            q = h(q)
            q = sdg(q)
            q = h(q)
            return measure(q)
        
        @guppy
        def test_tdg() -> bool:
            q = qubit()
            q = h(q)
            q = tdg(q)
            q = h(q)
            return measure(q)
        
        # Try to compile each function
        for func, gate_name in [(test_s, "S"), (test_t, "T"), 
                                (test_sdg, "Sdg"), (test_tdg, "Tdg")]:
            try:
                llvm_ir = compile_guppy_to_llvm(func)
                assert llvm_ir is not None
                assert f"__quantum__qis__{gate_name.lower()}__body" in llvm_ir
                print(f"✓ {gate_name} gate compiled successfully")
            except Exception as e:
                pytest.fail(f"{gate_name} gate compilation failed: {e}")
    
    def test_two_qubit_gates(self):
        """Test CY, CZ, CH gates."""
        
        @guppy
        def test_cy() -> tuple[bool, bool]:
            q1 = qubit()
            q2 = qubit()
            q1 = h(q1)
            q1, q2 = cy(q1, q2)
            return measure(q1), measure(q2)
        
        @guppy
        def test_cz() -> tuple[bool, bool]:
            q1 = qubit()
            q2 = qubit()
            q1 = h(q1)
            q2 = h(q2)
            q1, q2 = cz(q1, q2)
            q1 = h(q1)
            q2 = h(q2)
            return measure(q1), measure(q2)
        
        @guppy
        def test_ch() -> tuple[bool, bool]:
            q1 = qubit()
            q2 = qubit()
            q1 = x(q1)  # Set control to |1>
            q1, q2 = ch(q1, q2)
            return measure(q1), measure(q2)
        
        # Try to compile each function
        for func, gate in [(test_cy, "CY"), (test_cz, "CZ"), (test_ch, "CH")]:
            try:
                llvm_ir = compile_guppy_to_llvm(func)
                assert llvm_ir is not None
                
                # CH is a composite gate, check for its components
                if gate == "CH":
                    assert "__quantum__qis__ry__body" in llvm_ir
                    assert "__quantum__qis__cz__body" in llvm_ir
                else:
                    assert f"__quantum__qis__{gate.lower()}__body" in llvm_ir
                    
                print(f"✓ {gate} gate compiled successfully")
            except Exception as e:
                pytest.fail(f"{gate} gate compilation failed: {e}")
    
    def test_controlled_rotation(self):
        """Test CRZ gate with angle parameter."""
        
        @guppy
        def test_crz() -> tuple[bool, bool]:
            q1 = qubit()
            q2 = qubit()
            q1 = x(q1)  # Set control to |1>
            q2 = h(q2)
            q1, q2 = crz(q1, q2, pi/4)
            q2 = h(q2)
            return measure(q1), measure(q2)
        
        try:
            llvm_ir = compile_guppy_to_llvm(test_crz)
            assert llvm_ir is not None
            assert "__quantum__qis__crz__body" in llvm_ir
            print("✓ CRZ gate compiled successfully")
        except Exception as e:
            pytest.fail(f"CRZ gate compilation failed: {e}")
    
    def test_toffoli_gate(self):
        """Test Toffoli (CCX) gate."""
        
        @guppy
        def test_toffoli() -> tuple[bool, bool, bool]:
            q1 = qubit()
            q2 = qubit()
            q3 = qubit()
            q1 = x(q1)  # Set first control to |1>
            q2 = x(q2)  # Set second control to |1>
            q1, q2, q3 = toffoli(q1, q2, q3)
            return measure(q1), measure(q2), measure(q3)
        
        try:
            llvm_ir = compile_guppy_to_llvm(test_toffoli)
            assert llvm_ir is not None
            assert "__quantum__qis__ccx__body" in llvm_ir
            print("✓ Toffoli gate compiled successfully")
        except Exception as e:
            pytest.fail(f"Toffoli gate compilation failed: {e}")
    
    def test_combined_circuit(self):
        """Test a circuit combining multiple new gates."""
        
        @guppy
        def quantum_algorithm() -> tuple[bool, bool]:
            # Initialize qubits
            q1 = qubit()
            q2 = qubit()
            
            # Apply rotation gates
            q1 = rx(q1, pi/3)
            q1 = ry(q1, pi/4)
            
            # Apply Pauli gates
            q1 = s(q1)
            q2 = t(q2)
            
            # Apply controlled gates
            q1, q2 = cy(q1, q2)
            q1, q2 = crz(q1, q2, pi/6)
            
            # Final rotations
            q1 = sdg(q1)
            q2 = tdg(q2)
            
            return measure(q1), measure(q2)
        
        try:
            llvm_ir = compile_guppy_to_llvm(quantum_algorithm)
            assert llvm_ir is not None
            
            # Check for various gates in the output
            expected_gates = ["rx", "ry", "s", "t", "cy", "crz", "sdg", "tdg"]
            for gate in expected_gates:
                assert f"__quantum__qis__{gate}__body" in llvm_ir
            
            print("✓ Combined circuit compiled successfully with all new gates")
        except Exception as e:
            pytest.fail(f"Combined circuit compilation failed: {e}")


def run_tests():
    """Run tests and print summary."""
    print("="*60)
    print("Stage 1 Quantum Gates Test Suite")
    print("="*60)
    
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
    
    print("\n" + "="*60)
    print("✅ All Stage 1 quantum gates compiled successfully!")
    print("="*60)


if __name__ == "__main__":
    run_tests()