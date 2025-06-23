#!/usr/bin/env python3
"""Test library loading step by step"""

import sys
import os
sys.path.append("python/quantum-pecos/src")

def test_library_loading():
    """Test just the library loading without execution"""
    print("=== Testing library loading ===")
    
    qir_file = "/tmp/pecos_guppy_rust_7c0k0azv/guppy_func.ll"
    
    if not os.path.exists(qir_file):
        print("QIR file doesn't exist!")
        return False
    
    try:
        print("Step 1: Importing pecos_rslib...")
        from pecos_rslib import execute_qir, reset_qir_runtime
        print("✓ Import successful")
        
        print("Step 2: Calling reset_qir_runtime...")
        reset_qir_runtime()
        print("✓ Reset successful")
        
        print("Step 3: About to call execute_qir...")
        # This is where it likely fails
        result = execute_qir(qir_file, 1, 42, None, None, llvm_convention="hugr")
        print(f"✓ execute_qir successful: {result}")
        return True
        
    except Exception as e:
        print(f"✗ Failed at step: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    success = test_library_loading()
    print(f"\nFinal result: {'SUCCESS' if success else 'FAILED'}")