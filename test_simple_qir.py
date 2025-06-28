#!/usr/bin/env python3
"""
Test script for QIR interactive execution using the existing generated QIR file.

This bypasses the HUGR compilation step and directly tests the QIR runtime
with the generated file that contains HUGR immediate measurement patterns.
"""

import sys
import os

# Add the Python library path  
sys.path.insert(0, "/home/ciaranra/Repos/cl_projects/gup/PECOS/python/pecos-rslib/python")

try:
    from pecos_rslib import execute_llvm
    
    print("Testing QIR interactive execution...")
    
    # Use the existing HUGR-generated QIR file
    qir_path = "/home/ciaranra/Repos/cl_projects/gup/PECOS/guppy_generated.ll"
    
    if not os.path.exists(qir_path):
        print(f"ERROR: QIR file not found at {qir_path}")
        sys.exit(1)
    
    print(f"Loading QIR file: {qir_path}")
    
    try:
        # Execute the QIR file with multiple shots
        print("Running QIR program...")
        results = execute_llvm(qir_path, shots=100, seed=42, noise_probability=0.0, workers=1)
        
        print(f"Execution completed! Results type: {type(results)}")
        
        if isinstance(results, dict) and 'results' in results:
            measurement_results = results['results']
            print(f"Execution successful: {results.get('execution_successful', 'unknown')}")
            print(f"Number of shots: {results.get('shots', 'unknown')}")
            print(f"Measurement results length: {len(measurement_results)}")
            print(f"First 10 results: {measurement_results[:10]}")
            
            # Check if we're getting realistic measurements
            if all(r == False for r in measurement_results):
                print("WARNING: All measurements are False - interactive execution may not be working")
            else:
                print("SUCCESS: Got mixed measurement results - interactive execution is working!")
                
            # Count False and True values
            false_count = sum(1 for r in measurement_results if r == False)
            true_count = sum(1 for r in measurement_results if r == True)
            print(f"Results distribution: {false_count} False, {true_count} True")
            
            if true_count > 0:
                print("✅ QIR immediate measurements working with interactive execution!")
                print("✅ HUGR's immediate measurement model successfully integrated with PECOS!")
            else:
                print("❌ QIR immediate measurements still returning all False")
        else:
            print(f"Unexpected results format: {results}")
            
    except Exception as e:
        print(f"ERROR during QIR execution: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
    
except ImportError as e:
    print(f"Failed to import QIR executor: {e}")
    sys.exit(1)

print("Test completed.")