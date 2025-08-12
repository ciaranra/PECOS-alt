#!/usr/bin/env julia
# Copyright 2025 The PECOS Developers
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
# the License. You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

# Basic usage example for PECOS.jl
# This demonstrates the idiomatic Julia interface

using PECOS

# Get version information
println("PECOS Version: ", pecos_version())
println()

# Working with qubits
println("Creating qubits:")
qubits = [QubitId(i) for i = 0:4]
for q in qubits
    println("  ", q)
end
println()

# Future API preview (not yet implemented):
#
# # Run a quantum circuit
# qasm_code = """
# OPENQASM 2.0;
# include "qelib1.inc";
# qreg q[2];
# creg c[2];
# h q[0];
# cx q[0], q[1];
# measure q -> c;
# """
#
# results = run_qasm(qasm_code, shots=1000)
# println("Bell state results: ", results)
#
# # Create a stabilizer simulator
# sim = StabilizerSimulator(n_qubits=5)
# apply_gate!(sim, :H, QubitId(0))
# apply_gate!(sim, :CNOT, QubitId(0), QubitId(1))
# measure!(sim, qubits[1:2])

println("More features coming soon!")
