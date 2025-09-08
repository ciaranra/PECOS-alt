"""Simple test for rx(pi) gate behavior using existing API."""

import sys
import os

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "src"))

from collections import Counter

try:
    from pecos_rslib.qasm_sim import run_qasm

    # Test rx(pi) on |0⟩ - should give |1⟩
    qasm_rx_pi = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[1];
    creg c[1];
    rx(pi) q[0];
    measure q[0] -> c[0];
    """

    print("Testing rx(pi) gate...")
    results = run_qasm(qasm_rx_pi, shots=1000, seed=42)
    print(f"Results type: {type(results)}")

    # Convert ShotVec to dict if needed
    if hasattr(results, "to_dict"):
        results_dict = results.to_dict()
    else:
        results_dict = results

    print(f"Results dict: {type(results_dict)}")

    if "c" in results_dict:
        # Count outcomes
        counts = Counter(results_dict["c"])
        print(f"\nMeasurement outcomes: {counts}")

        # rx(pi) should flip |0⟩ to |1⟩, so we expect all 1s
        if counts.get(1, 0) > 900:
            print("✓ rx(pi) correctly flips |0⟩ to |1⟩")
        else:
            print("✗ rx(pi) is NOT working correctly - expected all 1s")
            print(f"Got {counts.get(1, 0)} ones out of 1000 shots")

    # Also test X gate for comparison
    qasm_x = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[1];
    creg c[1];
    x q[0];
    measure q[0] -> c[0];
    """

    print("\n\nTesting X gate for comparison...")
    results_x = run_qasm(qasm_x, shots=1000, seed=42)
    if hasattr(results_x, "to_dict"):
        results_x_dict = results_x.to_dict()
    else:
        results_x_dict = results_x
    if "c" in results_x_dict:
        counts_x = Counter(results_x_dict["c"])
        print(f"X gate outcomes: {counts_x}")

    # Test decomposed rx(pi)
    qasm_decomposed = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[1];
    creg c[1];
    h q[0];
    rz(pi) q[0];
    h q[0];
    measure q[0] -> c[0];
    """

    print("\n\nTesting decomposed rx(pi) (H-RZ(pi)-H)...")
    results_decomposed = run_qasm(qasm_decomposed, shots=1000, seed=42)
    if hasattr(results_decomposed, "to_dict"):
        results_decomposed_dict = results_decomposed.to_dict()
    else:
        results_decomposed_dict = results_decomposed
    if "c" in results_decomposed_dict:
        counts_decomposed = Counter(results_decomposed_dict["c"])
        print(f"Decomposed rx(pi) outcomes: {counts_decomposed}")

except Exception as e:
    print(f"Error importing or running: {e}")
    import traceback

    traceback.print_exc()
