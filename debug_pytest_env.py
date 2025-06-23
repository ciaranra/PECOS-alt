#!/usr/bin/env python3
"""Simulate pytest environment more closely"""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import h, measure, qubit, x
from pecos.frontends.run_guppy import run_guppy, get_guppy_backends
import gc

# Import test utilities
class GuppyPipelineTest:
    """Helper class for testing Guppy programs on both pipelines."""
    
    def __init__(self):
        self.backends = get_guppy_backends()
        
    def test_function_on_both_pipelines(self, func, shots: int = 10, seed: int = 42, **kwargs):
        """Test a Guppy function on both HUGR-LLVM and PMIR pipelines."""
        results = {}
        
        # Test HUGR-LLVM pipeline (rust backend)
        if self.backends.get("rust_backend", False):
            try:
                print(f"[TEST] Running {func} on rust backend")
                result = run_guppy(func, shots=shots, backend="rust", verbose=False, seed=seed, **kwargs)
                results["hugr_llvm"] = {
                    "success": True,
                    "result": result,
                    "backend": result.get("backend_used"),
                    "error": None
                }
                print(f"[TEST] Rust backend completed successfully")
            except Exception as e:
                print(f"[TEST] Rust backend failed: {e}")
                results["hugr_llvm"] = {
                    "success": False,
                    "result": None,
                    "backend": "rust",
                    "error": str(e)
                }
        
        # Force cleanup between backends
        gc.collect()
        
        # Test PMIR pipeline (external backend)
        try:
            print(f"[TEST] Running {func} on external backend")
            result = run_guppy(func, shots=shots, backend="external", verbose=False, seed=seed, **kwargs)
            results["pmir"] = {
                "success": True,
                "result": result,
                "backend": result.get("backend_used"),
                "error": None
            }
            print(f"[TEST] External backend completed successfully")
        except Exception as e:
            print(f"[TEST] External backend failed: {e}")
            results["pmir"] = {
                "success": False,
                "result": None,
                "backend": "external", 
                "error": str(e)
            }
            
        return results


class TestBasicQuantumOperations:
    """Test basic quantum gate operations on both pipelines."""
    
    def test_single_qubit_hadamard(self, pipeline_tester):
        """Test Hadamard gate on single qubit."""
        @guppy
        def hadamard_test() -> bool:
            q = qubit()
            h(q)
            return measure(q)
        
        results = pipeline_tester.test_function_on_both_pipelines(hadamard_test, shots=50)
        
        # Both pipelines should succeed
        assert results.get("hugr_llvm", {}).get("success", False), f"HUGR-LLVM failed: {results.get('hugr_llvm', {}).get('error')}"
        # PMIR might not be available on all systems
        if "pmir" in results:
            print(f"PMIR result: {results['pmir']}")
    
    def test_pauli_gates(self, pipeline_tester):
        """Test all Pauli gates (X, Y, Z)."""
        @guppy  
        def pauli_x_test() -> bool:
            q = qubit()
            x(q)  # Should flip |0⟩ to |1⟩
            return measure(q)
        
        # Test X gate
        results_x = pipeline_tester.test_function_on_both_pipelines(pauli_x_test, shots=100, seed=42)
        if results_x.get("hugr_llvm", {}).get("success"):
            ones_count = sum(results_x["hugr_llvm"]["result"]["results"])
            print(f"X gate ones count: {ones_count}/100")


# Simulate pytest execution
print("=== Simulating pytest environment ===")

# Create test class instance
test_instance = TestBasicQuantumOperations()

# Create pipeline tester (simulating fixture)
pipeline_tester = GuppyPipelineTest()

# Run tests in sequence like pytest would
print("\n[PYTEST] Running test_single_qubit_hadamard...")
try:
    test_instance.test_single_qubit_hadamard(pipeline_tester)
    print("[PYTEST] test_single_qubit_hadamard PASSED")
except Exception as e:
    print(f"[PYTEST] test_single_qubit_hadamard FAILED: {e}")
    import traceback
    traceback.print_exc()

# Force cleanup
gc.collect()

print("\n[PYTEST] Running test_pauli_gates...")
try:
    test_instance.test_pauli_gates(pipeline_tester)
    print("[PYTEST] test_pauli_gates PASSED")
except Exception as e:
    print(f"[PYTEST] test_pauli_gates FAILED: {e}")
    import traceback
    traceback.print_exc()

print("\n=== Simulation complete ===")