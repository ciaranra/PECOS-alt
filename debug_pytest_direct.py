#!/usr/bin/env python3
"""Run the actual test directly without pytest"""

import sys
sys.path.append("python/quantum-pecos/src")

# Import the test class
from python.tests.guppy.test_comprehensive_guppy_features import TestBasicQuantumOperations, GuppyPipelineTest

# Create an instance of the test class
test_instance = TestBasicQuantumOperations()

# Create the pipeline tester
pipeline_tester = GuppyPipelineTest()

print("=== Running test_single_qubit_hadamard directly ===")
try:
    test_instance.test_single_qubit_hadamard(pipeline_tester)
    print("✓ Test passed")
except Exception as e:
    print(f"✗ Test failed: {e}")
    import traceback
    traceback.print_exc()

print("\n=== Running test_pauli_gates directly ===")
try:
    test_instance.test_pauli_gates(pipeline_tester)
    print("✓ Test passed")
except Exception as e:
    print(f"✗ Test failed: {e}")
    import traceback
    traceback.print_exc()