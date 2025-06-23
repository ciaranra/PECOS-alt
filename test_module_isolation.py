#!/usr/bin/env python3
"""Test QIR execution with gradual module loading to identify conflict"""

import sys
import os
sys.path.append("python/quantum-pecos/src")

def test_basic_qir():
    """Test QIR without any extra modules"""
    print("=== Testing basic QIR execution ===")
    
    # Use a known working QIR file
    qir_file = "/tmp/pecos_guppy_rust_7c0k0azv/guppy_func.ll"
    
    if not os.path.exists(qir_file):
        print("QIR file doesn't exist!")
        return False
    
    from pecos_rslib import execute_qir, reset_qir_runtime
    
    try:
        reset_qir_runtime()
        result = execute_qir(qir_file, 1, 42, None, None, llvm_convention="hugr")
        print(f"Success! Result: {result}")
        return True
    except Exception as e:
        print(f"Failed: {e}")
        import traceback
        traceback.print_exc()
        return False

def test_with_numpy():
    """Test QIR after loading NumPy"""
    print("\n=== Testing with NumPy loaded ===")
    
    import numpy as np
    print(f"NumPy version: {np.__version__}")
    
    return test_basic_qir()

def test_with_scipy():
    """Test QIR after loading SciPy"""
    print("\n=== Testing with SciPy loaded ===")
    
    import scipy
    print(f"SciPy version: {scipy.__version__}")
    
    return test_basic_qir()

def test_with_pytest_modules():
    """Test QIR after loading common pytest modules"""
    print("\n=== Testing with pytest modules loaded ===")
    
    # Load common pytest dependencies
    import _pytest
    import pluggy
    import hypothesis
    
    print("Common pytest modules loaded")
    
    return test_basic_qir()

if __name__ == "__main__":
    # Test progression
    tests = [
        ("Basic QIR", test_basic_qir),
        ("With NumPy", test_with_numpy), 
        ("With SciPy", test_with_scipy),
        ("With pytest modules", test_with_pytest_modules)
    ]
    
    for test_name, test_func in tests:
        print(f"\n{'='*50}")
        print(f"Running: {test_name}")
        print(f"{'='*50}")
        
        success = test_func()
        print(f"Result: {'PASS' if success else 'FAIL'}")
        
        if not success:
            print(f"FAILED at: {test_name}")
            break
    
    print("\nModule isolation test completed")