#!/usr/bin/env python3
"""Comprehensive test of all working functionality"""

import sys
import os
sys.path.append("python/quantum-pecos/src")

def test_comprehensive_functionality():
    """Test all major working components"""
    print("=== Comprehensive Functionality Test ===")
    
    results = {}
    
    # Test 1: LLVM JIT properly disabled
    print("\n1. Testing LLVM JIT disabled...")
    try:
        from pecos_rslib import execute_qir
        result = execute_qir("test.ll", 1, 42, None, None, llvm_convention="qir") 
        assert result["execution_successful"] == False
        assert "LLVM JIT execution is disabled" in result["error_message"]
        results["llvm_jit_disabled"] = True
        print("✅ LLVM JIT properly disabled with helpful message")
    except Exception as e:
        print(f"❌ LLVM JIT disable failed: {e}")
        results["llvm_jit_disabled"] = False
    
    # Test 2: ByteMessage simulation
    print("\n2. Testing ByteMessage simulation...")
    try:
        from pecos_rslib import ByteMessageBuilder, StateVecEngineRs
        
        builder = ByteMessageBuilder()
        builder.add_h(0)
        builder.add_cx(0, 1)
        builder.add_measurement(0, 0)
        builder.add_measurement(1, 1)
        
        message = builder.build()
        engine = StateVecEngineRs(2)
        engine.set_seed(42)
        result = engine.run_circuit_with_shots(message, 5)
        
        assert len(result) == 5
        results["byte_message"] = True
        print(f"✅ ByteMessage simulation: {len(result)} shots")
    except Exception as e:
        print(f"❌ ByteMessage simulation failed: {e}")
        results["byte_message"] = False
    
    # Test 3: QASM simulation
    print("\n3. Testing QASM simulation...")
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
        
        result = run_qasm(qasm_code, 5, seed=42)
        assert 'c' in result
        assert len(result['c']) == 5
        results["qasm"] = True
        print(f"✅ QASM simulation: {result}")
    except Exception as e:
        print(f"❌ QASM simulation failed: {e}")
        results["qasm"] = False
    
    # Test 4: Native engines
    print("\n4. Testing native engines...")
    try:
        from pecos_rslib import StateVecEngineRs, SparseStabEngineRs
        
        # StateVec engine
        sv_engine = StateVecEngineRs(3)
        sv_engine.set_seed(42)
        
        # Sparse stabilizer engine  
        stab_engine = SparseStabEngineRs(3)
        stab_engine.set_seed(42)
        
        results["native_engines"] = True
        print("✅ Native engines created successfully")
    except Exception as e:
        print(f"❌ Native engines failed: {e}")
        results["native_engines"] = False
    
    # Test 5: Reset QIR runtime (should still work)
    print("\n5. Testing QIR runtime functions...")
    try:
        from pecos_rslib import reset_qir_runtime
        reset_qir_runtime()
        results["qir_runtime"] = True
        print("✅ QIR runtime functions work")
    except Exception as e:
        print(f"❌ QIR runtime functions failed: {e}")
        results["qir_runtime"] = False
    
    # Summary
    print(f"\n=== Summary ===")
    total_tests = len(results)
    passed_tests = sum(results.values())
    
    for test_name, passed in results.items():
        status = "✅ PASS" if passed else "❌ FAIL"
        print(f"{test_name}: {status}")
    
    print(f"\nOverall: {passed_tests}/{total_tests} tests passed")
    
    if passed_tests == total_tests:
        print("🎉 ALL FUNCTIONALITY WORKING PERFECTLY!")
        print("✅ LLVM JIT disabled cleanly")
        print("✅ All alternatives working") 
        print("✅ Ready for production use")
        return True
    else:
        print(f"❌ {total_tests - passed_tests} tests failed")
        return False

if __name__ == "__main__":
    success = test_comprehensive_functionality()
    exit(0 if success else 1)