#!/usr/bin/env python3
"""Test the performance improvement of the simplified QIR runtime."""

import time
import sys
import os

# Add path to PECOS
sys.path.append('python/quantum-pecos/src')

def test_basic_quantum_execution():
    """Test basic quantum execution with both conventions."""
    print("Testing simplified QIR runtime performance...")
    
    from guppylang import guppy
    from guppylang.std.quantum import h, measure, qubit
    from pecos.frontends import run_guppy
    
    @guppy
    def simple_circuit() -> bool:
        q = qubit()
        h(q)
        return measure(q)
    
    # Test HUGR convention (should be very fast now)
    print("\n1. Testing HUGR convention...")
    start_time = time.time()
    try:
        result = run_guppy(simple_circuit, shots=5, llvm_convention='hugr')
        hugr_time = time.time() - start_time
        print(f"   ✓ HUGR execution completed in {hugr_time:.3f} seconds")
        print(f"   Results: {result}")
    except Exception as e:
        hugr_time = time.time() - start_time
        print(f"   ✗ HUGR execution failed after {hugr_time:.3f} seconds: {e}")
    
    # Test QIR convention (should also be fast now)
    print("\n2. Testing QIR convention...")
    start_time = time.time()
    try:
        result = run_guppy(simple_circuit, shots=5, llvm_convention='qir')
        qir_time = time.time() - start_time
        print(f"   ✓ QIR execution completed in {qir_time:.3f} seconds")
        print(f"   Results: {result}")
    except Exception as e:
        qir_time = time.time() - start_time
        print(f"   ✗ QIR execution failed after {qir_time:.3f} seconds: {e}")
    
    # Performance evaluation
    print(f"\n3. Performance Summary:")
    if 'hugr_time' in locals():
        print(f"   HUGR convention: {hugr_time:.3f}s")
        if hugr_time < 2.0:
            print("   ✓ HUGR performance is good (< 2s)")
        else:
            print("   ✗ HUGR performance is still slow (>= 2s)")
    
    if 'qir_time' in locals():
        print(f"   QIR convention: {qir_time:.3f}s")
        if qir_time < 2.0:
            print("   ✓ QIR performance is good (< 2s)")
        else:
            print("   ✗ QIR performance is still slow (>= 2s)")

if __name__ == "__main__":
    test_basic_quantum_execution()