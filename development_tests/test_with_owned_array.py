#!/usr/bin/env python3
"""Test using with_owned pattern for selective measurement."""

# Test pattern using with_owned to temporarily take ownership
guppy_code = """from __future__ import annotations

from guppylang.decorator import guppy
from guppylang.std import quantum
from guppylang.std.builtins import array, owned, result
from guppylang.std.option import Option, some, nothing
from guppylang.std.mem import with_owned, mem_swap

@guppy
def measure_with_replacement(opt_q: Option[quantum.qubit] @ owned) -> tuple[bool, Option[quantum.qubit]]:
    if opt_q.is_some():
        q = opt_q.unwrap()
        bit = quantum.measure(q)
        return (bit, nothing())
    else:
        return (False, opt_q)

@guppy  
def main() -> None:
    # Create quantum array wrapped in Options
    q0 = some(quantum.qubit())
    q1 = some(quantum.qubit())  
    q2 = some(quantum.qubit())
    qubits = array(q0, q1, q2)
    
    # Apply gates first (this is tricky...)
    quantum.h(qubits[0].unwrap())  # Still problematic
    
    # Try using with_owned to temporarily take ownership of array element
    bit0 = with_owned(qubits[0], measure_with_replacement)
    
    result("bit0", bit0)
"""

print("=== Testing with_owned pattern ===")
print(guppy_code)

import tempfile
import importlib.util
from guppylang import guppy

# Write to temp file
with tempfile.NamedTemporaryFile(mode='w', suffix='.py', delete=False) as f:
    temp_file = f.name
    f.write(guppy_code)

try:
    # Import the module
    spec = importlib.util.spec_from_file_location("test_module", temp_file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    
    # Compile to HUGR
    print("Compiling to HUGR...")
    hugr_module = guppy.compile(module.main)
    print("HUGR compilation successful!")
    print("with_owned pattern WORKS!")
    
except Exception as e:
    print(f"HUGR compilation failed: {e}")

finally:
    import os
    os.unlink(temp_file)