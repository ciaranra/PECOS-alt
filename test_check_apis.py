#!/usr/bin/env python3
"""Check actual APIs of quantum engines"""

import sys
import os
import inspect
sys.path.append("python/quantum-pecos/src")

def inspect_apis():
    """Inspect the actual APIs"""
    print("=== Inspecting Available APIs ===")
    
    try:
        import pecos_rslib
        
        # Check ByteMessageBuilder
        print("ByteMessageBuilder methods:")
        builder = pecos_rslib.ByteMessageBuilder()
        methods = [m for m in dir(builder) if not m.startswith('_')]
        for method in methods:
            try:
                sig = inspect.signature(getattr(builder, method))
                print(f"  - {method}{sig}")
            except:
                print(f"  - {method} (no signature available)")
        
        # Check StateVecEngineRs
        print("\nStateVecEngineRs methods:")
        engine = pecos_rslib.StateVecEngineRs(2)
        methods = [m for m in dir(engine) if not m.startswith('_')]
        for method in methods:
            try:
                sig = inspect.signature(getattr(engine, method))
                print(f"  - {method}{sig}")
            except:
                print(f"  - {method} (no signature available)")
        
        # Check qasm_sim
        print("\nqasm_sim signature:")
        try:
            sig = inspect.signature(pecos_rslib.qasm_sim)
            print(f"  qasm_sim{sig}")
        except:
            print("  No signature available")
        
        # Check what run_qasm takes
        print("\nrun_qasm signature:")
        try:
            sig = inspect.signature(pecos_rslib.run_qasm)
            print(f"  run_qasm{sig}")
        except:
            print("  No signature available")
            
        return True
        
    except Exception as e:
        print(f"Failed: {e}")
        import traceback
        traceback.print_exc()
        return False

def test_correct_apis():
    """Test with correct APIs"""
    print("\n=== Testing with correct APIs ===")
    
    try:
        import pecos_rslib
        
        # Test run_qasm instead of qasm_sim
        print("Testing run_qasm...")
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
        
        result = pecos_rslib.run_qasm(qasm_code, {"shots": 1})
        print(f"✅ run_qasm successful: {result}")
        
        return True
        
    except Exception as e:
        print(f"❌ Failed: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    inspect_apis()
    test_correct_apis()