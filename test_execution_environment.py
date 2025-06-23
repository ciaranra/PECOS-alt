"""Test execution environment in pytest"""

import sys
import os
sys.path.append("python/quantum-pecos/src")

def test_execute_qir_with_debugging():
    """Test execute_qir with maximum debugging info"""
    
    # Use a known working QIR file
    qir_file = "/tmp/pecos_guppy_rust_7c0k0azv/guppy_func.ll"
    
    print("=== Environment Info ===")
    print(f"Python version: {sys.version}")
    print(f"Process ID: {os.getpid()}")
    print(f"Thread count: {len(sys.modules)}")
    print(f"In pytest: {'pytest' in sys.modules}")
    print(f"Extension modules loaded: {len([m for m in sys.modules if hasattr(sys.modules[m], '__file__') and sys.modules[m].__file__ and sys.modules[m].__file__.endswith('.so')])}")
    
    # List extension modules
    ext_modules = [m for m in sys.modules if hasattr(sys.modules[m], '__file__') and sys.modules[m].__file__ and sys.modules[m].__file__.endswith('.so')]
    if ext_modules:
        print("Extension modules:")
        for mod in ext_modules[:10]:  # First 10 only
            print(f"  {mod}")
    
    print(f"\n=== Testing QIR file: {qir_file} ===")
    if not os.path.exists(qir_file):
        print("QIR file doesn't exist!")
        return
    
    print("=== Importing pecos_rslib ===")
    from pecos_rslib import execute_qir, reset_qir_runtime
    
    print("=== Resetting QIR runtime ===")
    try:
        reset_qir_runtime()
        print("Reset successful")
    except Exception as e:
        print(f"Reset failed: {e}")
    
    print("=== Calling execute_qir ===")
    print("About to call execute_qir...")
    result = execute_qir(qir_file, 1, 42, None, None, llvm_convention="hugr")
    print(f"Success! Result: {result}")
    
    assert result.get("execution_successful", False)