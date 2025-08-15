#!/usr/bin/env python3
"""Test different measurement strategies for SLR → Guppy → HUGR."""

# Test all three measurement patterns
all_patterns_code = """from __future__ import annotations

from guppylang.decorator import guppy
from guppylang.std import quantum
from guppylang.std.builtins import array, owned, result

@guppy
def pattern1_measure_all() -> array[bool, 3]:
    \"\"\"Pattern 1: Measuring entire array at once\"\"\"
    q = array(quantum.qubit() for _ in range(3))
    quantum.h(q[0])
    quantum.cx(q[0], q[1])
    quantum.cx(q[1], q[2])
    
    # Measure entire array at once - most efficient
    return quantum.measure_array(q)

@guppy
def pattern2_selective_measure() -> array[bool, 5]:
    \"\"\"Pattern 2: Selective measurement with fresh qubit allocation\"\"\"
    q = array(quantum.qubit() for _ in range(5))
    
    # Apply some gates
    quantum.h(q[0])
    quantum.cx(q[0], q[1])
    
    # Unpack for selective measurement
    q0, q1, q2, q3, q4 = q
    
    # Measure first two
    c0 = quantum.measure(q0)
    c1 = quantum.measure(q1)
    
    # Continue with remaining qubits
    quantum.cx(q2, q3)
    
    # Allocate fresh qubits to replace measured ones
    q0_new = quantum.qubit()
    q1_new = quantum.qubit()
    
    # Use fresh qubits
    quantum.h(q0_new)
    quantum.cx(q0_new, q4)
    
    # Measure everything
    c2 = quantum.measure(q2)
    c3 = quantum.measure(q3)
    c4 = quantum.measure(q4)
    c0_new = quantum.measure(q0_new)
    c1_new = quantum.measure(q1_new)
    
    # Return original measurements (could also include new ones)
    return array(c0, c1, c2, c3, c4)

@guppy
def pattern3_qec_style() -> tuple[array[bool, 3], array[bool, 3]]:
    \"\"\"Pattern 3: QEC-style with ancilla measurements and reuse\"\"\"
    # Data qubits
    data = array(quantum.qubit() for _ in range(4))
    # Ancilla qubits
    ancilla = array(quantum.qubit() for _ in range(3))
    
    # Syndrome extraction
    quantum.h(ancilla[0])
    quantum.cx(ancilla[0], data[0])
    quantum.cx(ancilla[0], data[1])
    quantum.h(ancilla[0])
    
    quantum.h(ancilla[1])
    quantum.cx(ancilla[1], data[1])
    quantum.cx(ancilla[1], data[2])
    quantum.h(ancilla[1])
    
    quantum.h(ancilla[2])
    quantum.cx(ancilla[2], data[2])
    quantum.cx(ancilla[2], data[3])
    quantum.h(ancilla[2])
    
    # Measure ancillas (syndrome)
    syndrome = quantum.measure_array(ancilla)
    
    # In QEC, we often remeasure or reallocate ancillas
    # Allocate fresh ancillas for next round
    new_ancilla = array(quantum.qubit() for _ in range(3))
    
    # Do another round
    quantum.h(new_ancilla[0])
    quantum.cx(new_ancilla[0], data[0])
    quantum.cx(new_ancilla[0], data[1])
    quantum.h(new_ancilla[0])
    
    syndrome2 = quantum.measure_array(new_ancilla)
    
    # Clean up data qubits
    quantum.discard_array(data)
    
    return (syndrome, syndrome2)

@guppy
def main() -> None:
    # Test all patterns
    result1 = pattern1_measure_all()
    result("pattern1", result1)
    
    result2 = pattern2_selective_measure()
    result("pattern2", result2)
    
    syndrome1, syndrome2 = pattern3_qec_style()
    result("syndrome1", syndrome1)
    result("syndrome2", syndrome2)
"""

print("=== Testing all measurement strategies ===")
print("Pattern 1: Full array measurement → measure_array()")
print("Pattern 2: Selective measurement → unpack + fresh allocation")
print("Pattern 3: QEC-style → measure ancillas, allocate fresh ones")
print()

import tempfile
import importlib.util
from guppylang import guppy

# Write to temp file
with tempfile.NamedTemporaryFile(mode='w', suffix='.py', delete=False) as f:
    temp_file = f.name
    f.write(all_patterns_code)

try:
    # Import the module
    spec = importlib.util.spec_from_file_location("test_module", temp_file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    
    # Compile to HUGR
    print("Compiling to HUGR...")
    hugr_module = guppy.compile(module.main)
    print("✅ HUGR compilation successful!")
    print("\nAll three patterns work!")
    print("\nCode generation strategy:")
    print("1. Detect if measuring entire array → use measure_array()")
    print("2. Otherwise → unpack array before first measurement")
    print("3. After measurement → can allocate fresh qubits as needed")
    print("4. No need to 'repack' measured qubits - they're consumed")
    
except Exception as e:
    print(f"❌ HUGR compilation failed: {e}")
    import traceback
    traceback.print_exc()

finally:
    import os
    os.unlink(temp_file)