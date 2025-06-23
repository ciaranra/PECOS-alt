#!/usr/bin/env python3
"""Simulate pytest environment to reproduce segfault"""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import h, measure, qubit
from pecos.frontends.run_guppy import run_guppy
import gc

# Define multiple test functions like in the actual test file
@guppy
def hadamard_test() -> bool:
    q = qubit()
    h(q)
    return measure(q)

@guppy
def another_test() -> bool:
    q = qubit()
    return measure(q)

# Simulate the pipeline tester behavior
class PipelineTester:
    def test_function_on_both_pipelines(self, func, shots=50):
        """Simulate running on both pipelines like in the test"""
        results = {}
        
        # Run with rust backend
        print(f"[PIPELINE] Running {func} with rust backend")
        result_rust = run_guppy(func, shots=shots, backend="rust", seed=42)
        results["rust"] = result_rust
        
        # Force cleanup between runs
        gc.collect()
        
        # Run with external backend
        print(f"[PIPELINE] Running {func} with external backend")
        result_external = run_guppy(func, shots=shots, backend="external", seed=42)
        results["external"] = result_external
        
        return results

# Simulate running multiple tests in sequence
pipeline_tester = PipelineTester()

print("=== Test 1: Single qubit Hadamard ===")
try:
    results1 = pipeline_tester.test_function_on_both_pipelines(hadamard_test, shots=50)
    print("✓ Test 1 passed")
except Exception as e:
    print(f"✗ Test 1 failed: {e}")
    import traceback
    traceback.print_exc()

# Force cleanup
gc.collect()

print("\n=== Test 2: Another test ===")
try:
    results2 = pipeline_tester.test_function_on_both_pipelines(another_test, shots=50)
    print("✓ Test 2 passed")
except Exception as e:
    print(f"✗ Test 2 failed: {e}")
    import traceback
    traceback.print_exc()

print("\nAll tests completed.")