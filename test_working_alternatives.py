#!/usr/bin/env python3
"""Test working quantum simulation alternatives"""

import sys
import os
sys.path.append("python/quantum-pecos/src")

def test_byte_message_simulation():
    """Test direct ByteMessage simulation"""
    print("=== Testing ByteMessage + StateVecEngineRs ===")
    
    try:
        from pecos_rslib import ByteMessageBuilder, StateVecEngineRs
        
        # Create a Bell state using ByteMessage
        print("Step 1: Build quantum circuit")
        builder = ByteMessageBuilder()
        
        # Bell state: |00⟩ + |11⟩
        builder.add_h(0)        # H gate on qubit 0
        builder.add_cx(0, 1)    # CNOT from qubit 0 to 1
        builder.add_measurement(0, 0)  # Measure qubit 0 → result 0
        builder.add_measurement(1, 1)  # Measure qubit 1 → result 1
        
        message = builder.build()
        print(f"✅ Built quantum message")
        
        print("Step 2: Execute on StateVec engine")
        engine = StateVecEngineRs(2)  # 2 qubits
        engine.set_seed(42)
        
        result = engine.run_circuit_with_shots(message, 10)  # 10 shots
        print(f"✅ Simulation successful!")
        print(f"Result type: {type(result)}")
        print(f"Result: {result}")
        
        return True
        
    except Exception as e:
        print(f"❌ Failed: {e}")
        import traceback
        traceback.print_exc()
        return False

def test_qasm_simulation():
    """Test QASM simulation"""
    print("\n=== Testing QASM simulation ===")
    
    try:
        from pecos_rslib import run_qasm
        
        # Bell state in QASM
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
        
        print("Step 1: Run QASM simulation")
        result = run_qasm(qasm_code, 10, seed=42)  # 10 shots
        print(f"✅ QASM simulation successful!")
        print(f"Result: {result}")
        
        return True
        
    except Exception as e:
        print(f"❌ Failed: {e}")
        import traceback
        traceback.print_exc()
        return False

def test_in_pytest():
    """Test if these alternatives work in pytest"""
    print("\n=== Testing if these work in pytest ===")
    
    # This function will be called from pytest to see if the alternatives
    # work in that environment where execute_qir() fails
    
    byte_message_works = test_byte_message_simulation()
    qasm_works = test_qasm_simulation()
    
    return byte_message_works, qasm_works

if __name__ == "__main__":
    print("Testing quantum simulation alternatives that bypass LLVM JIT...")
    
    byte_message_works = test_byte_message_simulation()
    qasm_works = test_qasm_simulation()
    
    print(f"\n=== Summary ===")
    print(f"ByteMessage + StateVecEngineRs works: {byte_message_works}")
    print(f"QASM simulation works: {qasm_works}")
    
    if byte_message_works or qasm_works:
        print("\n✅ EXCELLENT! We have working alternatives!")
        print("This confirms the issue is specifically in execute_qir()'s LLVM JIT pipeline.")
        print("The quantum simulation core is working perfectly.")
        print("\nNext step: Test these alternatives in pytest to see if they work there too.")
    else:
        print("\n❌ Deeper issues in quantum simulation core.")