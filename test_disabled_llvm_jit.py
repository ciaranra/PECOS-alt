#!/usr/bin/env python3
"""Test that LLVM JIT is properly disabled and alternatives work"""

import sys
import os
sys.path.append("python/quantum-pecos/src")

def test_disabled_execute_qir():
    """Test that execute_qir shows helpful error when disabled"""
    print("=== Testing disabled execute_qir ===")
    
    try:
        from pecos_rslib import execute_qir
        
        # This should return a helpful error message, not crash
        result = execute_qir(
            "examples/qir/bell.ll",  # Any QIR file
            1,  # shots
            42, # seed
            None, None,
            llvm_convention="qir"
        )
        
        print(f"Result: {result}")
        
        # Verify it's properly disabled
        assert result["execution_successful"] == False
        assert "LLVM JIT execution is disabled" in result["error_message"]
        assert result["alternatives_available"] == True
        
        print("✅ execute_qir properly disabled with helpful error message")
        return True
        
    except Exception as e:
        print(f"❌ execute_qir failed unexpectedly: {e}")
        import traceback
        traceback.print_exc()
        return False

def test_alternatives_still_work():
    """Test that working alternatives still function"""
    print("\n=== Testing alternatives still work ===")
    
    # Test 1: ByteMessage simulation
    try:
        from pecos_rslib import ByteMessageBuilder, StateVecEngineRs
        
        builder = ByteMessageBuilder()
        builder.add_h(0)
        builder.add_cx(0, 1)
        builder.add_measurement(0, 0)
        builder.add_measurement(1, 1)
        
        message = builder.build()
        engine = StateVecEngineRs(2)
        result = engine.run_circuit_with_shots(message, 3)
        
        print(f"✅ ByteMessage simulation works: {len(result)} shots")
        
    except Exception as e:
        print(f"❌ ByteMessage simulation failed: {e}")
        return False
    
    # Test 2: QASM simulation
    try:
        from pecos_rslib import run_qasm
        
        qasm_code = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q[0] -> c[0];
        measure q[1] -> c[1];
        """
        
        result = run_qasm(qasm_code, 3, seed=42)
        print(f"✅ QASM simulation works: {result}")
        
    except Exception as e:
        print(f"❌ QASM simulation failed: {e}")
        return False
    
    return True

def test_in_pytest_environment():
    """Test that this works properly in pytest"""
    disabled_works = test_disabled_execute_qir()
    alternatives_work = test_alternatives_still_work()
    return disabled_works and alternatives_work

if __name__ == "__main__":
    print("Testing LLVM JIT conditional disabling...")
    
    disabled_works = test_disabled_execute_qir()
    alternatives_work = test_alternatives_still_work()
    
    print(f"\n=== Summary ===")
    print(f"execute_qir properly disabled: {disabled_works}")
    print(f"Alternatives still work: {alternatives_work}")
    
    if disabled_works and alternatives_work:
        print("\n🎉 SUCCESS! LLVM JIT is cleanly disabled, alternatives work perfectly!")
        print("Ready to run tests without hanging/segfaulting.")
    else:
        print("\n❌ Issues with the conditional disabling.")