OPENQASM 2.0;
include "qelib1.inc";

// Create registers with different bit widths
qreg q[4];
creg c[2];  // 2-bit register
creg d[3];  // 3-bit register

// Bell state on first two qubits
h q[0];
cx q[0], q[1];

// Another bell state on last two qubits
h q[2];
cx q[2], q[3];

// Measure
measure q[0] -> c[0];
measure q[1] -> c[1];
measure q[2] -> d[0];
measure q[3] -> d[1];
// d[2] remains 0