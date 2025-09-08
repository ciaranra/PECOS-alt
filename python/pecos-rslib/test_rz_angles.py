"""Test RZ gate with different angles to isolate the issue."""

import sys
import os

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "src"))

from collections import Counter
import math

try:
    from pecos_rslib.qasm_sim import run_qasm

    # First, verify Z gate works (RZ(pi) should be equivalent to Z)
    print("Test Z gate (should give phase flip on |+⟩)")
    qasm_z = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[1];
    creg c[1];
    h q[0];
    z q[0];
    h q[0];
    measure q[0] -> c[0];
    """
    results = run_qasm(qasm_z, shots=100, seed=42)
    if hasattr(results, "to_dict"):
        results = results.to_dict()
    print(f"H-Z-H (should be X): {Counter(results['c'])}")

    # Test RZ with different angles on |+⟩ state
    print("\nTesting RZ on |+⟩ state (H|0⟩):")
    angles = [
        ("0", 0, "Should stay |0⟩"),
        ("pi/4", math.pi / 4, "Should be intermediate"),
        ("pi/2", math.pi / 2, "Should be intermediate"),
        ("pi", math.pi, "Should flip to |1⟩"),
        ("3.14159", 3.14159, "Should flip to |1⟩"),
        ("-pi", -math.pi, "Should flip to |1⟩"),
    ]

    for angle_str, _angle_val, desc in angles:
        # Test with qelib1 rz
        qasm = f"""
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        h q[0];
        rz({angle_str}) q[0];
        h q[0];
        measure q[0] -> c[0];
        """

        results = run_qasm(qasm, shots=100, seed=42)
        if hasattr(results, "to_dict"):
            results = results.to_dict()

        counts = Counter(results["c"])
        prob_1 = counts.get(1, 0) / 100

        print(f"\nH-rz({angle_str})-H: {desc}")
        print(f"  Counts: {counts}")
        print(f"  P(|1⟩) = {prob_1:.2f}")

        # For RZ(pi), we expect it to flip the state
        if angle_str in ["pi", "3.14159", "-pi"] and prob_1 < 0.9:
            print("  ❌ FAIL: Should flip to |1⟩")

    # Test if the issue is specific to the rz gate or RZ native gate
    print("\n\nComparing qelib1 rz vs native RZ:")

    # Using qelib1 rz
    qasm_qelib = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[1];
    creg c[1];
    h q[0];
    rz(3.14159) q[0];
    h q[0];
    measure q[0] -> c[0];
    """

    # Using native RZ
    qasm_native = """
    OPENQASM 2.0;
    qreg q[1];
    creg c[1];
    H q[0];
    RZ(3.14159) q[0];
    H q[0];
    measure q[0] -> c[0];
    """

    results_qelib = run_qasm(qasm_qelib, shots=100, seed=42)
    if hasattr(results_qelib, "to_dict"):
        results_qelib = results_qelib.to_dict()

    try:
        results_native = run_qasm(qasm_native, shots=100, seed=42)
        if hasattr(results_native, "to_dict"):
            results_native = results_native.to_dict()
        print(f"qelib1 rz: {Counter(results_qelib['c'])}")
        print(f"Native RZ: {Counter(results_native['c'])}")
    except Exception as e:
        print(f"Native RZ test failed: {e}")
        print(f"qelib1 rz: {Counter(results_qelib['c'])}")

except Exception as e:
    print(f"Error: {e}")
    import traceback

    traceback.print_exc()
