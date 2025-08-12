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

# Demonstration of PECOS.jl functionality

using PECOS

println("PECOS.jl Demonstration")
println("======================\n")

# 1. Version information
println("1. Version Information:")
version = pecos_version()
println("   $version")
println()

# 2. Creating qubits
println("2. Creating Qubits:")
qubits = QubitId[]
for i = 0:4
    q = QubitId(i)
    push!(qubits, q)
    println("   Created: $q")
end
println()

# 3. Error handling
println("3. Error Handling:")
try
    q_invalid = QubitId(-1)
catch e
    println("   Caught expected error: $e")
end
println()

# 4. Working with collections
println("4. Working with Qubit Collections:")
println("   Number of qubits: $(length(qubits))")
println("   First qubit: $(qubits[1])")
println("   Last qubit: $(qubits[end])")
println()

# 5. Direct FFI demonstration
println("5. Direct FFI Calls (Advanced):")
result =
    ccall((:add_two_numbers, PECOS_julia_jll.libpecos_julia), Int64, (Int64, Int64), 10, 32)
println("   10 + 32 = $result (called via FFI)")
println()

println("PECOS.jl is ready for quantum error correction simulations!")
