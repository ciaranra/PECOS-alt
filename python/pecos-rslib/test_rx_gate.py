"""Test rx(pi) gate behavior."""

import sys
sys.path.append('/home/ciaranra/Repos/cl_projects/gup/PECOS-alt/python/pecos-rslib/src')

from pecos_rslib import qasm_engine
from pecos_rslib.programs import QasmProgram

# Test 1: rx(pi) on |0⟩ should give |1⟩
qasm_rx_pi = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[1];
creg c[1];
rx(pi) q[0];
measure q[0] -> c[0];
"""

# Test 2: X gate (for comparison) on |0⟩ should give |1⟩
qasm_x = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[1];
creg c[1];
x q[0];
measure q[0] -> c[0];
"""

# Test 3: Decomposed rx(pi) explicitly
qasm_rx_decomposed = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[1];
creg c[1];
h q[0];
rz(pi) q[0];
h q[0];
measure q[0] -> c[0];
"""

# Run tests
print("Testing rx(pi) gate...")
results_rx = qasm_engine().program(QasmProgram.from_string(qasm_rx_pi)).to_sim().seed(42).run(1000)
print(f"rx(pi) results: {results_rx}")

print("\nTesting X gate...")
results_x = qasm_engine().program(QasmProgram.from_string(qasm_x)).to_sim().seed(42).run(1000)
print(f"X gate results: {results_x}")

print("\nTesting decomposed rx(pi)...")
results_decomposed = qasm_engine().program(QasmProgram.from_string(qasm_rx_decomposed)).to_sim().seed(42).run(1000)
print(f"Decomposed rx(pi) results: {results_decomposed}")

# Check if rx(pi) flips the qubit correctly
if 'c' in results_rx:
    count_ones = sum(1 for bit in results_rx['c'] if bit == '1')
    count_zeros = sum(1 for bit in results_rx['c'] if bit == '0')
    print(f"\nrx(pi) measurement counts: |0⟩: {count_zeros}, |1⟩: {count_ones}")
    
    if count_ones > 900:  # Should be ~1000 for ideal case
        print("✓ rx(pi) correctly flips |0⟩ to |1⟩")
    else:
        print("✗ rx(pi) is NOT working correctly - should flip |0⟩ to |1⟩")

# Also test rx(pi/2) for creating superposition
qasm_rx_pi2 = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[1];
creg c[1];
rx(pi/2) q[0];
measure q[0] -> c[0];
"""

print("\n\nTesting rx(pi/2) gate (should create superposition)...")
results_rx_pi2 = qasm_engine().program(QasmProgram.from_string(qasm_rx_pi2)).to_sim().seed(42).run(1000)
if 'c' in results_rx_pi2:
    count_ones = sum(1 for bit in results_rx_pi2['c'] if bit == '1')
    count_zeros = sum(1 for bit in results_rx_pi2['c'] if bit == '0')
    print(f"rx(pi/2) measurement counts: |0⟩: {count_zeros}, |1⟩: {count_ones}")
    print(f"Should be roughly 50/50 for superposition state")