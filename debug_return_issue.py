#!/usr/bin/env python3
"""Debug the return value issue more precisely"""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import h, measure, qubit
import gc
import pathlib

@guppy
def hadamard_test() -> bool:
    q = qubit()
    h(q)
    return measure(q)

# Test with minimal code path
def test_execute_qir_directly():
    """Test calling execute_qir directly to isolate the issue"""
    from pecos.frontends.guppy_frontend import GuppyFrontend
    from pecos_rslib import execute_qir, reset_qir_runtime
    import pathlib
    
    print("=== Testing execute_qir directly ===")
    
    # Create frontend and compile
    frontend = GuppyFrontend(use_rust_backend=True, llvm_convention="hugr")
    qir_file = frontend.compile_function(hadamard_test)
    print(f"Compiled to: {qir_file}")
    print(f"QIR file type: {type(qir_file)}")
    print(f"QIR file exists: {qir_file.exists()}")
    
    # Reset runtime
    print("\nResetting QIR runtime...")
    reset_qir_runtime()
    
    # Call execute_qir directly
    print("\nCalling execute_qir...")
    try:
        # Store path as string first
        qir_path_str = str(qir_file)
        print(f"QIR path string: {qir_path_str}")
        
        result = execute_qir(
            qir_path_str,
            50,  # shots
            42,  # seed
            None,  # noise_probability
            None,  # workers
            llvm_convention="hugr"
        )
        print(f"execute_qir returned successfully")
        print(f"Result type: {type(result)}")
        print(f"Result keys: {list(result.keys()) if isinstance(result, dict) else 'Not a dict'}")
        
        # Try accessing result values
        if isinstance(result, dict):
            for key, value in result.items():
                print(f"  {key}: {type(value)}")
                if key == "results" and isinstance(value, list):
                    print(f"    Length: {len(value)}")
                    if value:
                        print(f"    First result: {value[0]}")
        
        # Force garbage collection
        print("\nForcing garbage collection...")
        gc.collect()
        print("GC completed successfully")
        
        return result
        
    except Exception as e:
        print(f"execute_qir failed: {e}")
        import traceback
        traceback.print_exc()
        raise

# Test creating dictionary like run_guppy does
def test_dictionary_creation():
    """Test creating the return dictionary"""
    print("\n=== Testing dictionary creation ===")
    
    # Simulate the values
    results = [True, False, True, False] * 12 + [True, True]  # 50 results
    shots = 50
    function_name = "hadamard_test"
    backend_used = "rust"
    compilation_time = 0.01
    execution_time = 3.0
    qir_file = pathlib.Path("/tmp/test.ll")
    backend_info = {"backend": "rust"}
    
    print("Creating dictionary...")
    return_dict = {}
    
    steps = [
        ("results", results),
        ("shots", shots),
        ("function_name", function_name),
        ("backend_used", backend_used),
        ("compilation_time", compilation_time),
        ("execution_time", execution_time),
        ("qir_file", str(qir_file)),
        ("backend_info", backend_info),
    ]
    
    for key, value in steps:
        print(f"Adding '{key}'...")
        return_dict[key] = value
        print(f"  Success. Dict now has {len(return_dict)} items")
    
    print("Dictionary created successfully")
    return return_dict

# Run tests
try:
    dict_result = test_dictionary_creation()
    print(f"\n✓ Dictionary creation test passed")
except Exception as e:
    print(f"\n✗ Dictionary creation test failed: {e}")

try:
    qir_result = test_execute_qir_directly()
    print(f"\n✓ execute_qir test passed")
except Exception as e:
    print(f"\n✗ execute_qir test failed: {e}")

print("\nAll tests completed.")