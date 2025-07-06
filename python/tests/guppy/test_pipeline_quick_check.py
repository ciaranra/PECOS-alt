#!/usr/bin/env python3
"""Quick test to check pipeline capabilities without hanging."""

import sys
from pathlib import Path

import pytest

sys.path.append("python/quantum-pecos/src")

try:
    from guppylang import guppy
    from guppylang.std.quantum import h, measure, qubit, cx
    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False

try:
    from pecos.frontends.run_guppy import run_guppy, get_guppy_backends
    PECOS_FRONTEND_AVAILABLE = True
except ImportError:
    PECOS_FRONTEND_AVAILABLE = False


@pytest.mark.skipif(not GUPPY_AVAILABLE or not PECOS_FRONTEND_AVAILABLE, 
                    reason="Dependencies not available")
def test_quick_pipeline_check():
    """Quick test of both pipelines with minimal shots."""
    
    @guppy
    def test_h() -> bool:
        q = qubit()
        h(q)
        return measure(q)
    
    backends = get_guppy_backends()
    print(f"\nAvailable backends: {backends}")
    
    # Test just one simple circuit with 1 shot
    results = {}
    
    # Test HUGR-LLVM
    if backends.get("rust_backend", False):
        print("\nTesting HUGR-LLVM backend...")
        try:
            result = run_guppy(test_h, shots=1, backend="rust", verbose=False)
            results["hugr_llvm"] = "✅ PASS"
            print(f"  Result: {result.get('results', [])}")
        except Exception as e:
            results["hugr_llvm"] = f"❌ FAIL: {str(e)[:50]}"
            print(f"  Error: {str(e)[:100]}")
    
    # Test PHIR
    print("\nTesting PHIR backend...")
    try:
        result = run_guppy(test_h, shots=1, backend="external", verbose=False)
        results["phir"] = "✅ PASS"
        print(f"  Result: {result.get('results', [])}")
    except Exception as e:
        results["phir"] = f"❌ FAIL: {str(e)[:50]}"
        print(f"  Error: {str(e)[:100]}")
    
    # Summary
    print("\n" + "="*50)
    print("QUICK CHECK SUMMARY:")
    for backend, status in results.items():
        print(f"  {backend}: {status}")
    print("="*50)


if __name__ == "__main__":
    test_quick_pipeline_check()