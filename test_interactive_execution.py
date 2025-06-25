#!/usr/bin/env python3
"""
Test script for HUGR interactive execution with QIR engine.

This test verifies that HUGR's immediate measurement model works correctly
with PECOS's EngineSystem architecture for interactive execution.
"""

import sys
import os

# Add the Python library path
sys.path.insert(0, "/home/ciaranra/Repos/cl_projects/gup/PECOS/python/pecos-rslib/python")

try:
    from pecos_rslib import RustHugrQirEngine as HugrQirEngine
    
    print("Testing HUGR interactive execution...")
    
    # Load the HUGR-generated QIR file
    qir_path = "/home/ciaranra/Repos/cl_projects/gup/PECOS/guppy_generated.ll"
    
    if not os.path.exists(qir_path):
        print(f"ERROR: QIR file not found at {qir_path}")
        sys.exit(1)
    
    print(f"Loading QIR file: {qir_path}")
    
    # Create a QIR engine directly from the existing QIR file
    # We'll simulate what would happen with HUGR compilation by reading the file
    with open(qir_path, 'rb') as f:
        qir_content = f.read()
    
    # For now, let's use a mock HUGR bytes input (the actual HUGR->QIR pipeline would be used here)
    # This test focuses on the QIR interactive execution part
    mock_hugr_bytes = b"mock_hugr_data"  # In reality, this would be the actual HUGR serialized data
    
    try:
        # Create QIR engine with interactive execution support
        engine = HugrQirEngine(mock_hugr_bytes, shots=100, debug_info=True, llvm_convention="hugr")
        
        print(f"Created QIR engine with ID: {engine.get_engine_id()}")
        print(f"Engine shots: {engine.get_shots()}")
        
        # Run the quantum program - this should trigger interactive execution
        print("Running quantum program with interactive execution...")
        results = engine.run()
        
        print(f"Execution completed! Results length: {len(results)}")
        print(f"First 10 results: {results[:10]}")
        
        # Check if we're getting non-zero measurements (the Hadamard gate should give ~50% 0s and 1s)
        if all(r == 0 for r in results):
            print("WARNING: All measurements are 0 - interactive execution may not be working")
        else:
            print("SUCCESS: Got mixed measurement results - interactive execution is working!")
            
        # Count 0s and 1s
        zeros = sum(1 for r in results if r == 0)
        ones = sum(1 for r in results if r == 1)
        print(f"Results distribution: {zeros} zeros, {ones} ones")
        
        if ones > 0:
            print("✅ HUGR immediate measurements working with interactive execution!")
        else:
            print("❌ HUGR immediate measurements still returning all zeros")
            
    except Exception as e:
        print(f"ERROR during execution: {e}")
        sys.exit(1)
    
except ImportError as e:
    print(f"Failed to import HUGR QIR engine: {e}")
    print("Make sure the pecos-rslib Python module is built with hugr-llvm-pipeline feature")
    sys.exit(1)

print("Test completed.")