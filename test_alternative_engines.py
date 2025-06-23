#!/usr/bin/env python3
"""Test alternative QIR execution engines"""

import sys
import os
sys.path.append("python/quantum-pecos/src")

def test_hugr_qir_engine():
    """Test HUGR-specific QIR engine"""
    print("=== Testing HUGR QIR engine ===")
    
    qir_file = "/tmp/pecos_guppy_rust_7c0k0azv/guppy_func.ll"
    
    if not os.path.exists(qir_file):
        print("QIR file doesn't exist!")
        return False
    
    try:
        from pecos_rslib import hugr_qir, reset_qir_runtime
        
        print("Step 1: Reset runtime")
        reset_qir_runtime()
        
        print("Step 2: Try hugr_qir function")
        # Try the HUGR-specific function with minimal parameters
        result = hugr_qir(qir_file, 1)  # Just file and shots
        print(f"✓ hugr_qir succeeded: {result}")
        return True
        
    except Exception as e:
        print(f"✗ hugr_qir failed: {e}")
        import traceback
        traceback.print_exc()
        return False

def test_create_qir_engine():
    """Test creating QIR engine separately"""
    print("\n=== Testing create_qir_engine_from_hugr_rust ===")
    
    qir_file = "/tmp/pecos_guppy_rust_7c0k0azv/guppy_func.ll"
    
    try:
        from pecos_rslib import create_qir_engine_from_hugr_rust, reset_qir_runtime
        
        print("Step 1: Reset runtime")
        reset_qir_runtime()
        
        print("Step 2: Create QIR engine")
        engine = create_qir_engine_from_hugr_rust(qir_file)
        print(f"✓ Engine created: {engine}")
        
        print("Step 3: Try to execute with engine")
        # See what methods the engine has
        print(f"Engine methods: {[m for m in dir(engine) if not m.startswith('_')]}")
        
        return True
        
    except Exception as e:
        print(f"✗ create_qir_engine failed: {e}")
        import traceback
        traceback.print_exc()
        return False

def test_inspect_functions():
    """Inspect function signatures"""
    print("\n=== Inspecting function signatures ===")
    
    try:
        import pecos_rslib
        import inspect
        
        # Check execute_qir signature
        print("execute_qir signature:")
        sig = inspect.signature(pecos_rslib.execute_qir)
        print(f"  {sig}")
        
        # Check hugr_qir signature  
        print("\nhugr_qir signature:")
        sig = inspect.signature(pecos_rslib.hugr_qir)
        print(f"  {sig}")
        
        # Check create_qir_engine signature
        print("\ncreate_qir_engine_from_hugr_rust signature:")
        sig = inspect.signature(pecos_rslib.create_qir_engine_from_hugr_rust)
        print(f"  {sig}")
        
        return True
        
    except Exception as e:
        print(f"Failed to inspect: {e}")
        return False

if __name__ == "__main__":
    test_inspect_functions()
    test_hugr_qir_engine()
    test_create_qir_engine()