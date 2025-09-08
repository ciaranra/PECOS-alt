"""Test gate sequence to debug rx(pi) issue."""

import sys
import os

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "src"))

from collections import Counter
import math

try:
    from pecos_rslib.qasm_sim import run_qasm

    # Test different angle values
    angles = [
        ("0", "0"),
        ("pi/4", f"{math.pi/4}"),
        ("pi/2", f"{math.pi/2}"),
        ("pi", f"{math.pi}"),
        ("3*pi/2", f"{3*math.pi/2}"),
        ("2*pi", f"{2*math.pi}"),
    ]

    print("Testing RX gate with different angles:")
    print("=" * 50)

    for angle_str, angle_val in angles:
        qasm = f"""
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        rx({angle_str}) q[0];
        measure q[0] -> c[0];
        """

        results = run_qasm(qasm, shots=1000, seed=42)
        if hasattr(results, "to_dict"):
            results = results.to_dict()

        counts = Counter(results["c"])
        prob_1 = counts.get(1, 0) / 1000

        # Calculate expected probability of |1⟩
        # rx(θ) |0⟩ = cos(θ/2)|0⟩ - i*sin(θ/2)|1⟩
        # P(|1⟩) = sin²(θ/2)
        expected_prob_1 = math.sin(float(angle_val) / 2) ** 2

        print(f"\nrx({angle_str}):")
        print(f"  Measured P(|1⟩) = {prob_1:.3f}")
        print(f"  Expected P(|1⟩) = {expected_prob_1:.3f}")
        print(f"  Counts: {counts}")

        if angle_str == "pi":
            if prob_1 < 0.9:
                print("  ❌ FAIL: rx(pi) should flip to |1⟩")
            else:
                print("  ✓ PASS")

    # Also test the decomposition directly
    print("\n" + "=" * 50)
    print("Testing decomposition H-RZ(θ)-H:")

    for angle_str, _ in [("pi/2", ""), ("pi", ""), ("3*pi/2", "")]:
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

        results = run_qasm(qasm, shots=1000, seed=42)
        if hasattr(results, "to_dict"):
            results = results.to_dict()

        counts = Counter(results["c"])
        print(f"\nH-RZ({angle_str})-H: {counts}")

except Exception as e:
    print(f"Error: {e}")
    import traceback

    traceback.print_exc()
