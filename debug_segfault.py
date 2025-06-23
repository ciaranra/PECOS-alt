#!/usr/bin/env python3
"""Debug script to isolate the segfault issue"""

import sys
sys.path.append("python/quantum-pecos/src")

print("Starting debug script...")

try:
    from guppylang import guppy
    from guppylang.std.quantum import h, measure, qubit
    print("✓ Guppy imports successful")
except ImportError as e:
    print(f"✗ Guppy import failed: {e}")
    sys.exit(1)

try:
    from pecos.frontends.run_guppy import run_guppy
    print("✓ run_guppy import successful")
except ImportError as e:
    print(f"✗ run_guppy import failed: {e}")
    sys.exit(1)

# Define a simple quantum function
@guppy
def simple_test() -> bool:
    """Simplest possible quantum program"""
    q = qubit()
    return measure(q)

print("\n=== Running simple test ===")
print("Function:", simple_test)
print("Expected: Should measure |0⟩ (False) every time")

try:
    # Run with minimal parameters
    result = run_guppy(simple_test, shots=1, verbose=True, backend="rust")
    print(f"\n✓ Test completed successfully!")
    print(f"Result: {result}")
except Exception as e:
    print(f"\n✗ Test failed with exception: {type(e).__name__}: {e}")
    import traceback
    traceback.print_exc()

print("\nScript completed.")