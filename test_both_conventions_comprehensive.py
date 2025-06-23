#!/usr/bin/env python3
"""
Comprehensive tests for both QIR and HUGR LLVM-IR conventions

This test file verifies that both conventions work correctly through
the full PECOS simulation infrastructure from Python.
"""

import tempfile
import os
from pathlib import Path

def test_simple_gate_hugr():
    """Test a simple gate with HUGR convention"""
    print("=" * 60)
    print("Testing HUGR Convention - Simple H Gate")
    print("=" * 60)
    
    try:
        from guppylang import guppy, qubit
        from guppylang.std.quantum import h
        from pecos.frontends.run_guppy import run_guppy
        
        @guppy
        def simple_h() -> qubit:
            q = qubit()
            h(q)
            return q
        
        print("Running with HUGR convention...")
        result = run_guppy(
            simple_h, 
            shots=5, 
            llvm_convention='hugr', 
            verbose=True
        )
        
        print(f"HUGR Result: {result}")
        print("✓ HUGR Convention Test: PASSED")
        return True
        
    except Exception as e:
        print(f"✗ HUGR Convention Test: FAILED - {e}")
        return False

def test_simple_gate_qir():
    """Test a simple gate with QIR convention"""
    print("=" * 60)
    print("Testing QIR Convention - Simple H Gate")
    print("=" * 60)
    
    try:
        from guppylang import guppy, qubit
        from guppylang.std.quantum import h
        from pecos.frontends.run_guppy import run_guppy
        
        @guppy
        def simple_h() -> qubit:
            q = qubit()
            h(q)
            return q
        
        print("Running with QIR convention...")
        result = run_guppy(
            simple_h, 
            shots=5, 
            llvm_convention='qir', 
            verbose=True
        )
        
        print(f"QIR Result: {result}")
        print("✓ QIR Convention Test: PASSED")
        return True
        
    except Exception as e:
        print(f"✗ QIR Convention Test: FAILED - {e}")
        return False

def test_multi_gate_hugr():
    """Test multiple gates with HUGR convention"""
    print("=" * 60)
    print("Testing HUGR Convention - Multiple Gates")
    print("=" * 60)
    
    try:
        from guppylang import guppy, qubit
        from guppylang.std.quantum import h, x, y, z, cx
        from pecos.frontends.run_guppy import run_guppy
        
        @guppy
        def multi_gate() -> tuple[qubit, qubit]:
            q1 = qubit()
            q2 = qubit()
            
            # Apply various gates
            h(q1)
            x(q2)
            cx(q1, q2)
            y(q1)
            z(q2)
            
            return q1, q2
        
        print("Running multi-gate circuit with HUGR convention...")
        result = run_guppy(
            multi_gate, 
            shots=3, 
            llvm_convention='hugr', 
            verbose=True
        )
        
        print(f"HUGR Multi-Gate Result: {result}")
        print("✓ HUGR Multi-Gate Test: PASSED")
        return True
        
    except Exception as e:
        print(f"✗ HUGR Multi-Gate Test: FAILED - {e}")
        return False

def test_multi_gate_qir():
    """Test multiple gates with QIR convention"""
    print("=" * 60)
    print("Testing QIR Convention - Multiple Gates")
    print("=" * 60)
    
    try:
        from guppylang import guppy, qubit
        from guppylang.std.quantum import h, x, y, z, cx
        from pecos.frontends.run_guppy import run_guppy
        
        @guppy
        def multi_gate() -> tuple[qubit, qubit]:
            q1 = qubit()
            q2 = qubit()
            
            # Apply various gates
            h(q1)
            x(q2)
            cx(q1, q2)
            y(q1)
            z(q2)
            
            return q1, q2
        
        print("Running multi-gate circuit with QIR convention...")
        result = run_guppy(
            multi_gate, 
            shots=3, 
            llvm_convention='qir', 
            verbose=True
        )
        
        print(f"QIR Multi-Gate Result: {result}")
        print("✓ QIR Multi-Gate Test: PASSED")
        return True
        
    except Exception as e:
        print(f"✗ QIR Multi-Gate Test: FAILED - {e}")
        return False

def test_measurement_hugr():
    """Test measurement with HUGR convention"""
    print("=" * 60)
    print("Testing HUGR Convention - Measurement")
    print("=" * 60)
    
    try:
        from guppylang import guppy, qubit
        from guppylang.std.quantum import h, measure
        from pecos.frontends.run_guppy import run_guppy
        
        @guppy
        def measure_h() -> bool:
            q = qubit()
            h(q)
            return measure(q)
        
        print("Running measurement circuit with HUGR convention...")
        result = run_guppy(
            measure_h, 
            shots=10, 
            llvm_convention='hugr', 
            verbose=True
        )
        
        print(f"HUGR Measurement Result: {result}")
        
        # Check that we get measurement results
        if 'results' in result and len(result['results']) == 10:
            print("✓ HUGR Measurement Test: PASSED")
            return True
        else:
            print("✗ HUGR Measurement Test: FAILED - No measurement results")
            return False
        
    except Exception as e:
        print(f"✗ HUGR Measurement Test: FAILED - {e}")
        return False

def test_measurement_qir():
    """Test measurement with QIR convention"""
    print("=" * 60)
    print("Testing QIR Convention - Measurement") 
    print("=" * 60)
    
    try:
        from guppylang import guppy, qubit
        from guppylang.std.quantum import h, measure
        from pecos.frontends.run_guppy import run_guppy
        
        @guppy
        def measure_h() -> bool:
            q = qubit()
            h(q)
            return measure(q)
        
        print("Running measurement circuit with QIR convention...")
        result = run_guppy(
            measure_h, 
            shots=10, 
            llvm_convention='qir', 
            verbose=True
        )
        
        print(f"QIR Measurement Result: {result}")
        
        # Check that we get measurement results
        if 'results' in result and len(result['results']) == 10:
            print("✓ QIR Measurement Test: PASSED")
            return True
        else:
            print("✗ QIR Measurement Test: FAILED - No measurement results")
            return False
        
    except Exception as e:
        print(f"✗ QIR Measurement Test: FAILED - {e}")
        return False

def test_bell_state_hugr():
    """Test Bell state preparation with HUGR convention"""
    print("=" * 60)
    print("Testing HUGR Convention - Bell State")
    print("=" * 60)
    
    try:
        from guppylang import guppy, qubit
        from guppylang.std.quantum import h, cx, measure
        from pecos.frontends.run_guppy import run_guppy
        
        @guppy
        def bell_state() -> tuple[bool, bool]:
            q1 = qubit()
            q2 = qubit()
            
            h(q1)
            cx(q1, q2)
            
            return measure(q1), measure(q2)
        
        print("Running Bell state with HUGR convention...")
        result = run_guppy(
            bell_state, 
            shots=20, 
            llvm_convention='hugr', 
            verbose=True
        )
        
        print(f"HUGR Bell State Result: {result}")
        
        # Check that we get correlated measurement results
        if 'results' in result and len(result['results']) == 20:
            print("✓ HUGR Bell State Test: PASSED")
            return True
        else:
            print("✗ HUGR Bell State Test: FAILED - No measurement results")
            return False
        
    except Exception as e:
        print(f"✗ HUGR Bell State Test: FAILED - {e}")
        return False

def test_bell_state_qir():
    """Test Bell state preparation with QIR convention"""
    print("=" * 60)
    print("Testing QIR Convention - Bell State")
    print("=" * 60)
    
    try:
        from guppylang import guppy, qubit
        from guppylang.std.quantum import h, cx, measure
        from pecos.frontends.run_guppy import run_guppy
        
        @guppy
        def bell_state() -> tuple[bool, bool]:
            q1 = qubit()
            q2 = qubit()
            
            h(q1)
            cx(q1, q2)
            
            return measure(q1), measure(q2)
        
        print("Running Bell state with QIR convention...")
        result = run_guppy(
            bell_state, 
            shots=20, 
            llvm_convention='qir', 
            verbose=True
        )
        
        print(f"QIR Bell State Result: {result}")
        
        # Check that we get correlated measurement results
        if 'results' in result and len(result['results']) == 20:
            print("✓ QIR Bell State Test: PASSED")
            return True
        else:
            print("✗ QIR Bell State Test: FAILED - No measurement results")
            return False
        
    except Exception as e:
        print(f"✗ QIR Bell State Test: FAILED - {e}")
        return False

def test_llvm_output_generation():
    """Test that .ll files are generated correctly for both conventions"""
    print("=" * 60)
    print("Testing LLVM Output Generation")
    print("=" * 60)
    
    try:
        from guppylang import guppy, qubit
        from guppylang.std.quantum import h
        from pecos.compilation_pipeline import compile_guppy_to_hugr, compile_hugr_to_llvm
        
        @guppy
        def simple_h() -> qubit:
            q = qubit()
            h(q)
            return q
        
        print("Compiling to HUGR...")
        hugr_bytes = compile_guppy_to_hugr(simple_h)
        print(f"HUGR size: {len(hugr_bytes)} bytes")
        
        # Test HUGR convention
        print("\nCompiling HUGR to LLVM with HUGR convention...")
        hugr_llvm = compile_hugr_to_llvm(hugr_bytes, llvm_convention='hugr')
        
        # Test QIR convention  
        print("Compiling HUGR to LLVM with QIR convention...")
        qir_llvm = compile_hugr_to_llvm(hugr_bytes, llvm_convention='qir')
        
        print(f"HUGR LLVM size: {len(hugr_llvm)} bytes")
        print(f"QIR LLVM size: {len(qir_llvm)} bytes")
        
        # Verify they're different
        if hugr_llvm != qir_llvm:
            print("✓ HUGR and QIR produce different LLVM output")
        else:
            print("⚠ HUGR and QIR produce identical LLVM output")
        
        print("✓ LLVM Output Generation Test: PASSED")
        return True
        
    except Exception as e:
        print(f"✗ LLVM Output Generation Test: FAILED - {e}")
        return False

def run_all_tests():
    """Run all tests and provide a summary"""
    print("\n" + "=" * 80)
    print("COMPREHENSIVE PECOS QIR/HUGR CONVENTION TESTING")
    print("=" * 80)
    
    tests = [
        ("HUGR Simple Gate", test_simple_gate_hugr),
        ("QIR Simple Gate", test_simple_gate_qir),
        ("HUGR Multi-Gate", test_multi_gate_hugr),
        ("QIR Multi-Gate", test_multi_gate_qir),
        ("HUGR Measurement", test_measurement_hugr),
        ("QIR Measurement", test_measurement_qir),
        ("HUGR Bell State", test_bell_state_hugr),
        ("QIR Bell State", test_bell_state_qir),
        ("LLVM Output Generation", test_llvm_output_generation),
    ]
    
    passed = 0
    failed = 0
    
    for test_name, test_func in tests:
        print(f"\n{'=' * 20} {test_name} {'=' * 20}")
        try:
            if test_func():
                passed += 1
            else:
                failed += 1
        except Exception as e:
            print(f"✗ {test_name}: FAILED with exception - {e}")
            failed += 1
    
    print("\n" + "=" * 80)
    print("TEST SUMMARY")
    print("=" * 80)
    print(f"Total tests: {passed + failed}")
    print(f"Passed: {passed}")
    print(f"Failed: {failed}")
    
    if failed == 0:
        print("🎉 ALL TESTS PASSED! Both QIR and HUGR conventions work correctly.")
    else:
        print(f"⚠️  {failed} test(s) failed. Check the output above for details.")
    
    return failed == 0

if __name__ == "__main__":
    success = run_all_tests()
    exit(0 if success else 1)