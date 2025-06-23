#!/usr/bin/env python3
"""Detailed debugging of the segfault location"""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import h, measure, qubit
from pecos.frontends.run_guppy import run_guppy
import traceback

@guppy
def hadamard_test() -> bool:
    q = qubit()
    h(q)
    return measure(q)

# Create a modified version of run_guppy with extensive logging
def debug_run_guppy(guppy_function, shots=1, backend=None, llvm_convention="hugr", verbose=False, seed=None, **kwargs):
    """Debug version of run_guppy with extensive logging"""
    import time
    from pecos.frontends.guppy_frontend import GuppyFrontend
    from pecos_rslib import execute_qir, reset_qir_runtime
    
    print(f"[DEBUG] Starting run_guppy")
    print(f"[DEBUG] Function: {guppy_function}")
    print(f"[DEBUG] Shots: {shots}, Backend: {backend}")
    
    # Check if this is a Guppy function
    function_name = getattr(
        guppy_function,
        "__name__",
        getattr(guppy_function, "name", str(guppy_function)),
    )
    print(f"[DEBUG] Function name: {function_name}")
    
    # Create frontend
    try:
        print("[DEBUG] Creating GuppyFrontend...")
        frontend = GuppyFrontend(
            use_rust_backend=True if backend == "rust" else None,
            llvm_convention=llvm_convention,
            **kwargs,
        )
        print("[DEBUG] Frontend created successfully")
    except Exception as e:
        print(f"[DEBUG] Frontend creation failed: {e}")
        raise
    
    # Get backend info
    backend_info = frontend.get_backend_info()
    backend_used = backend_info["backend"]
    print(f"[DEBUG] Using backend: {backend_used}")
    
    # Compile function
    print("[DEBUG] Starting compilation...")
    start_time = time.time()
    try:
        qir_file = frontend.compile_function(guppy_function)
        compilation_time = time.time() - start_time
        print(f"[DEBUG] Compilation successful in {compilation_time:.4f}s")
        print(f"[DEBUG] QIR file type: {type(qir_file)}")
        print(f"[DEBUG] QIR file value: {qir_file}")
        print(f"[DEBUG] QIR file str: {str(qir_file)}")
    except Exception as e:
        print(f"[DEBUG] Compilation failed: {e}")
        traceback.print_exc()
        raise
    
    # Reset QIR runtime
    print("[DEBUG] Resetting QIR runtime...")
    try:
        reset_qir_runtime()
        print("[DEBUG] QIR runtime reset successful")
    except Exception as e:
        print(f"[DEBUG] QIR runtime reset failed: {e}")
    
    # Execute QIR
    print("[DEBUG] Starting QIR execution...")
    execution_start = time.time()
    
    actual_convention = llvm_convention
    if backend_used == "external":
        actual_convention = "qir"
        print(f"[DEBUG] Overriding convention to 'qir' for external backend")
    
    print(f"[DEBUG] Calling execute_qir with:")
    print(f"  - qir_file: {str(qir_file)}")
    print(f"  - shots: {shots}")
    print(f"  - seed: {seed}")
    print(f"  - convention: {actual_convention}")
    
    try:
        qir_result = execute_qir(
            str(qir_file),
            shots,
            seed,
            None,  # noise_probability
            None,  # workers
            llvm_convention=actual_convention
        )
        print(f"[DEBUG] execute_qir returned successfully")
        print(f"[DEBUG] Result type: {type(qir_result)}")
        print(f"[DEBUG] Result keys: {list(qir_result.keys()) if isinstance(qir_result, dict) else 'Not a dict'}")
    except Exception as e:
        print(f"[DEBUG] execute_qir failed: {e}")
        traceback.print_exc()
        raise
    
    # Process results
    print("[DEBUG] Processing results...")
    if qir_result.get("execution_successful", False):
        results = qir_result.get("results", [])
        execution_time = time.time() - execution_start
        print(f"[DEBUG] Execution successful in {execution_time:.4f}s")
        print(f"[DEBUG] Got {len(results)} results")
        
        # Build return dictionary carefully
        print("[DEBUG] Building return dictionary...")
        return_dict = {}
        
        print("[DEBUG] Adding 'results'...")
        return_dict["results"] = results
        print("[DEBUG] Added 'results' successfully")
        
        print("[DEBUG] Adding 'shots'...")
        return_dict["shots"] = shots
        print("[DEBUG] Added 'shots' successfully")
        
        print("[DEBUG] Adding 'function_name'...")
        return_dict["function_name"] = function_name
        print("[DEBUG] Added 'function_name' successfully")
        
        print("[DEBUG] Adding 'backend_used'...")
        return_dict["backend_used"] = backend_used
        print("[DEBUG] Added 'backend_used' successfully")
        
        print("[DEBUG] Adding 'compilation_time'...")
        return_dict["compilation_time"] = compilation_time
        print("[DEBUG] Added 'compilation_time' successfully")
        
        print("[DEBUG] Adding 'execution_time'...")
        return_dict["execution_time"] = execution_time
        print("[DEBUG] Added 'execution_time' successfully")
        
        print("[DEBUG] Adding 'qir_file'...")
        print(f"[DEBUG] qir_file type before str(): {type(qir_file)}")
        try:
            qir_file_str = str(qir_file)
            print(f"[DEBUG] str(qir_file) successful: {qir_file_str}")
            return_dict["qir_file"] = qir_file_str
            print("[DEBUG] Added 'qir_file' successfully")
        except Exception as e:
            print(f"[DEBUG] str(qir_file) failed: {e}")
            traceback.print_exc()
            return_dict["qir_file"] = "<conversion failed>"
        
        print("[DEBUG] Adding 'backend_info'...")
        return_dict["backend_info"] = backend_info
        print("[DEBUG] Added 'backend_info' successfully")
        
        print("[DEBUG] Return dictionary built successfully")
        print("[DEBUG] About to return...")
        
        # Try to trigger garbage collection to see if that causes issues
        import gc
        print("[DEBUG] Forcing garbage collection...")
        gc.collect()
        print("[DEBUG] Garbage collection complete")
        
        return return_dict
    else:
        error_details = qir_result.get("error", "Unknown error")
        print(f"[DEBUG] QIR execution failed: {error_details}")
        raise RuntimeError(f"QIR execution failed: {error_details}")

# Run the debug version
print("=== Running debug version ===")
try:
    result = debug_run_guppy(hadamard_test, shots=50, backend="rust", verbose=True, seed=42)
    print("✓ Debug run completed successfully")
    print(f"Result keys: {list(result.keys())}")
except Exception as e:
    print(f"✗ Debug run failed: {e}")
    traceback.print_exc()

print("\n=== Now trying original run_guppy ===")
try:
    result = run_guppy(hadamard_test, shots=50, backend="rust", verbose=True, seed=42)
    print("✓ Original run_guppy completed successfully")
except Exception as e:
    print(f"✗ Original run_guppy failed: {e}")
    traceback.print_exc()