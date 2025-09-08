"""Test RZ gate behavior to debug rx(pi) issue."""

import sys
import os

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "src"))

from collections import Counter

try:
    from pecos_rslib.qasm_sim import run_qasm

    # Test 1: RZ(pi) on |0⟩ should stay |0⟩
    print("Test 1: RZ(pi) on |0⟩")
    qasm1 = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[1];
    creg c[1];
    rz(pi) q[0];
    measure q[0] -> c[0];
    """
    results1 = run_qasm(qasm1, shots=100, seed=42)
    if hasattr(results1, "to_dict"):
        results1 = results1.to_dict()
    print(f"RZ(pi) on |0⟩: {Counter(results1['c'])}")

    # Test 2: RZ(pi) on |+⟩ should give |-⟩
    print("\nTest 2: RZ(pi) on |+⟩")
    qasm2 = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[1];
    creg c[1];
    h q[0];
    rz(pi) q[0];
    h q[0];
    measure q[0] -> c[0];
    """
    results2 = run_qasm(qasm2, shots=100, seed=42)
    if hasattr(results2, "to_dict"):
        results2 = results2.to_dict()
    print(f"H-RZ(pi)-H (should be X): {Counter(results2['c'])}")

    # Test 3: Just H gate
    print("\nTest 3: Just H gate")
    qasm3 = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[1];
    creg c[1];
    h q[0];
    measure q[0] -> c[0];
    """
    results3 = run_qasm(qasm3, shots=100, seed=42)
    if hasattr(results3, "to_dict"):
        results3 = results3.to_dict()
    print(f"H gate: {Counter(results3['c'])}")

    # Test 4: H-H (should be identity)
    print("\nTest 4: H-H (should be identity)")
    qasm4 = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[1];
    creg c[1];
    h q[0];
    h q[0];
    measure q[0] -> c[0];
    """
    results4 = run_qasm(qasm4, shots=100, seed=42)
    if hasattr(results4, "to_dict"):
        results4 = results4.to_dict()
    print(f"H-H: {Counter(results4['c'])}")

    # Test 5: Native RZ vs gate rz
    print("\nTest 5: Testing if native RZ is being called")
    qasm5 = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[1];
    creg c[1];
    RZ(pi) q[0];  // Direct native gate call
    measure q[0] -> c[0];
    """
    try:
        results5 = run_qasm(qasm5, shots=100, seed=42)
        if hasattr(results5, "to_dict"):
            results5 = results5.to_dict()
        print(f"Native RZ(pi): {Counter(results5['c'])}")
    except Exception as e:
        print(f"Native RZ failed: {e}")

except Exception as e:
    print(f"Error: {e}")
    import traceback

    traceback.print_exc()
