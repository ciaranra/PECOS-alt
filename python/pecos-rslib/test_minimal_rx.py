"""Minimal test to isolate rx(pi) issue."""

import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'src'))

from collections import Counter

try:
    from pecos_rslib.qasm_sim import run_qasm
    
    # Test 1: Just measure |0⟩
    print("Test 1: Measure |0⟩")
    qasm1 = """
    OPENQASM 2.0;
    qreg q[1];
    creg c[1];
    measure q[0] -> c[0];
    """
    results1 = run_qasm(qasm1, shots=10, seed=42)
    if hasattr(results1, 'to_dict'):
        results1 = results1.to_dict()
    print(f"Result: {Counter(results1['c'])}")
    
    # Test 2: X then measure (no qelib1)
    print("\nTest 2: X then measure (using native X)")
    qasm2 = """
    OPENQASM 2.0;
    qreg q[1];
    creg c[1];
    X q[0];
    measure q[0] -> c[0];
    """
    try:
        results2 = run_qasm(qasm2, shots=10, seed=42)
        if hasattr(results2, 'to_dict'):
            results2 = results2.to_dict()
        print(f"Result: {Counter(results2['c'])}")
    except Exception as e:
        print(f"Native X failed: {e}")
    
    # Test 3: With qelib1, use x gate
    print("\nTest 3: x gate with qelib1")
    qasm3 = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[1];
    creg c[1];
    x q[0];
    measure q[0] -> c[0];
    """
    results3 = run_qasm(qasm3, shots=10, seed=42)
    if hasattr(results3, 'to_dict'):
        results3 = results3.to_dict()
    print(f"Result: {Counter(results3['c'])}")
    
    # Test 4: Native H gate
    print("\nTest 4: Native H gate")
    qasm4 = """
    OPENQASM 2.0;
    qreg q[1];
    creg c[1];
    H q[0];
    measure q[0] -> c[0];
    """
    try:
        results4 = run_qasm(qasm4, shots=100, seed=42)
        if hasattr(results4, 'to_dict'):
            results4 = results4.to_dict()
        print(f"Result: {Counter(results4['c'])}")
    except Exception as e:
        print(f"Native H failed: {e}")
    
    # Test 5: qelib1 h gate
    print("\nTest 5: qelib1 h gate")
    qasm5 = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[1];
    creg c[1];
    h q[0];
    measure q[0] -> c[0];
    """
    results5 = run_qasm(qasm5, shots=100, seed=42)
    if hasattr(results5, 'to_dict'):
        results5 = results5.to_dict()
    print(f"Result: {Counter(results5['c'])}")
    
    # Test 6: Native RZ gate
    print("\nTest 6: Native RZ(pi) on |+⟩")
    qasm6 = """
    OPENQASM 2.0;
    qreg q[1];
    creg c[1];
    H q[0];
    RZ(pi) q[0];
    H q[0];
    measure q[0] -> c[0];
    """
    try:
        results6 = run_qasm(qasm6, shots=10, seed=42)
        if hasattr(results6, 'to_dict'):
            results6 = results6.to_dict()
        print(f"Result: {Counter(results6['c'])}")
    except Exception as e:
        print(f"Native gates failed: {e}")
    
    # Test 7: Check if pi constant works
    print("\nTest 7: Explicit pi value")
    qasm7 = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[1];
    creg c[1];
    rx(3.14159265359) q[0];
    measure q[0] -> c[0];
    """
    results7 = run_qasm(qasm7, shots=10, seed=42)
    if hasattr(results7, 'to_dict'):
        results7 = results7.to_dict()
    print(f"Result: {Counter(results7['c'])}")
    
except Exception as e:
    print(f"Error: {e}")
    import traceback
    traceback.print_exc()