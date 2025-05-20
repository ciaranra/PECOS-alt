OPENQASM 2.0;
include "qelib1.inc";

// Simple deterministic circuit for testing purposes
// It creates superposition and entanglement with a predictable result pattern

qreg q[3];
creg c[3];

// Initialize q[0] to |1⟩ state
x q[0];

// Create superposition on q[1]
h q[1];

// Apply controlled-not operations to create entanglement
cx q[0], q[1];  // CNOT with q[0] as control, q[1] as target
cx q[1], q[2];  // CNOT with q[1] as control, q[2] as target

// Measure all qubits
measure q -> c;