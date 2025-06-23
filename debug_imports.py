#!/usr/bin/env python3
"""Check imports"""

import sys
sys.path.append("python/quantum-pecos/src")

print("Testing imports...")

try:
    from pecos_rslib import HUGR_LLVM_PIPELINE_AVAILABLE, PMIR_PIPELINE_AVAILABLE
    print(f"✓ pecos_rslib imports successful")
    print(f"  HUGR_LLVM_PIPELINE_AVAILABLE: {HUGR_LLVM_PIPELINE_AVAILABLE}")
    print(f"  PMIR_PIPELINE_AVAILABLE: {PMIR_PIPELINE_AVAILABLE}")
except ImportError as e:
    print(f"✗ pecos_rslib import failed: {e}")
except Exception as e:
    print(f"✗ Unexpected error: {type(e).__name__}: {e}")

print("\nChecking if there are any issues with multiple imports...")
    
# Try importing and running in sequence like the test does
try:
    from pecos.frontends.run_guppy import run_guppy, get_guppy_backends
    print("✓ run_guppy imports successful")
    
    backends = get_guppy_backends()
    print(f"  Backends: {backends}")
except Exception as e:
    print(f"✗ Error: {e}")