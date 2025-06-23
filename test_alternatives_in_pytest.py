#!/usr/bin/env python3
"""Test working alternatives in pytest environment"""

import sys
import os
sys.path.append("python/quantum-pecos/src")

def test_byte_message_in_pytest():
    """Test ByteMessage simulation in pytest"""
    print("=== Testing ByteMessage in pytest ===")
    
    from pecos_rslib import ByteMessageBuilder, StateVecEngineRs
    
    # Create a Bell state
    builder = ByteMessageBuilder()
    builder.add_h(0)        # H gate on qubit 0
    builder.add_cx(0, 1)    # CNOT from qubit 0 to 1
    builder.add_measurement(0, 0)  # Measure qubit 0 → result 0
    builder.add_measurement(1, 1)  # Measure qubit 1 → result 1
    
    message = builder.build()
    
    # Execute on StateVec engine
    engine = StateVecEngineRs(2)  # 2 qubits
    engine.set_seed(42)
    
    result = engine.run_circuit_with_shots(message, 5)  # 5 shots
    print(f"ByteMessage simulation result: {result}")
    
    # Verify we got valid Bell state results (should be [0,0] or [1,1])
    valid_results = all(shot[0][1] == shot[1][1] for shot in result)  # Both qubits same
    assert valid_results, f"Invalid Bell state results: {result}"
    
    print("✅ ByteMessage simulation works in pytest!")
    return True

def test_qasm_in_pytest():
    """Test QASM simulation in pytest"""
    print("=== Testing QASM in pytest ===")
    
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
    
    result = run_qasm(qasm_code, 5, seed=42)  # 5 shots
    print(f"QASM simulation result: {result}")
    
    # Verify we got results
    assert 'c' in result, f"Missing 'c' register in result: {result}"
    assert len(result['c']) == 5, f"Expected 5 shots, got: {len(result['c'])}"
    
    print("✅ QASM simulation works in pytest!")
    return True

def test_broken_execute_qir():
    """Confirm execute_qir still fails"""
    print("=== Confirming execute_qir still fails ===")
    
    qir_file = "examples/qir/bell.ll"
    
    if not os.path.exists(qir_file):
        print(f"❌ {qir_file} not found, skipping")
        return False
    
    try:
        from pecos_rslib import execute_qir, reset_qir_runtime
        
        reset_qir_runtime()
        
        # This should hang/segfault
        print("About to call execute_qir (this will likely hang/crash)...")
        result = execute_qir(qir_file, 1, 42, None, None, llvm_convention="qir")
        print(f"❌ execute_qir unexpectedly succeeded: {result}")
        return True
        
    except Exception as e:
        print(f"✅ execute_qir failed as expected: {e}")
        return False