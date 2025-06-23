#!/usr/bin/env python3
"""Test bypassing Python overhead by calling Rust functions directly"""

import sys
import os
sys.path.append("python/quantum-pecos/src")

def test_minimal_rust_execution():
    """Test with minimal Rust calls, bypassing Python QIR frontend"""
    print("=== Testing minimal Rust execution ===")
    
    qir_file = "/tmp/pecos_guppy_rust_7c0k0azv/guppy_func.ll"
    
    if not os.path.exists(qir_file):
        print("QIR file doesn't exist!")
        return False
    
    try:
        # Import just the core Rust functions
        from pecos_rslib import reset_qir_runtime
        
        print("Step 1: Reset QIR runtime")
        reset_qir_runtime()
        
        print("Step 2: Try to import minimal QIR execution")
        # See if there are more direct Rust functions available
        import pecos_rslib
        print("Available functions:", [attr for attr in dir(pecos_rslib) if not attr.startswith('_')])
        
        return True
        
    except Exception as e:
        print(f"Failed: {e}")
        import traceback
        traceback.print_exc()
        return False

def test_compare_rust_vs_python():
    """Compare what Rust does vs what Python does"""
    print("\n=== Comparing execution paths ===")
    
    # Check what exactly the Python QIR execution does
    qir_file = "/tmp/pecos_guppy_rust_7c0k0azv/guppy_func.ll"
    
    print(f"QIR file exists: {os.path.exists(qir_file)}")
    print(f"QIR file size: {os.path.getsize(qir_file) if os.path.exists(qir_file) else 'N/A'} bytes")
    
    # Read the QIR content to understand what we're trying to execute
    if os.path.exists(qir_file):
        with open(qir_file, 'r') as f:
            content = f.read()
            print(f"QIR content preview (first 200 chars):")
            print(content[:200])
            print("...")
            
            # Check for specific patterns that might cause issues
            print(f"Contains 'quantum' functions: {'__quantum__' in content}")
            print(f"Entry point functions: {content.count('define ')}")
    
    return True

if __name__ == "__main__":
    test_minimal_rust_execution()
    test_compare_rust_vs_python()