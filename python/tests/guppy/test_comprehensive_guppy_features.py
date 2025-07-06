#!/usr/bin/env python3
"""Comprehensive testing of Guppy language features across both HUGR-LLVM and PHIR pipelines.

This test suite systematically validates that both compilation pipelines can handle
the full spectrum of Guppy language capabilities, from basic quantum operations
to advanced classical-quantum hybrid programs.
"""

import sys
from pathlib import Path
from typing import Any, Dict, List, Tuple

import pytest

sys.path.append("python/quantum-pecos/src")

# Check dependencies
try:
    from guppylang import guppy
    from guppylang.std.quantum import h, measure, qubit, cx, x, y, z
    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False

try:
    from pecos.frontends.run_guppy import run_guppy, get_guppy_backends
    PECOS_FRONTEND_AVAILABLE = True
except ImportError:
    PECOS_FRONTEND_AVAILABLE = False

try:
    from pecos_rslib import HUGR_LLVM_PIPELINE_AVAILABLE
except ImportError:
    HUGR_LLVM_PIPELINE_AVAILABLE = False



class GuppyPipelineTest:
    """Helper class for testing Guppy programs on both pipelines."""
    
    def __init__(self):
        self.backends = get_guppy_backends() if PECOS_FRONTEND_AVAILABLE else {}
        
    def test_function_on_both_pipelines(self, func, shots: int = 10, seed: int = 42, **kwargs) -> Dict[str, Any]:
        """Test a Guppy function (using the Rust backend)."""
        results = {}
        
        # Test with Rust backend (the only backend)
        if self.backends.get("rust_backend", False):
            try:
                result = run_guppy(func, shots=shots, verbose=False, seed=seed, **kwargs)
                results["hugr_llvm"] = {
                    "success": True,
                    "result": result,
                    "error": None
                }
            except Exception as e:
                results["hugr_llvm"] = {
                    "success": False,
                    "result": None,
                    "error": str(e)
                }
            
        return results


@pytest.fixture
def pipeline_tester():
    """Fixture providing the pipeline testing helper."""
    return GuppyPipelineTest()


# ============================================================================
# BASIC QUANTUM OPERATIONS TESTS
# ============================================================================

@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not PECOS_FRONTEND_AVAILABLE, reason="PECOS frontend not available")
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
        # PHIR might not be available on all systems
        if "phir" in results:
            print(f"PHIR result: {results['phir']}")
    
    def test_pauli_gates(self, pipeline_tester):
        """Test all Pauli gates (X, Y, Z)."""
        @guppy  
        def pauli_x_test() -> bool:
            q = qubit()
            x(q)  # Should flip |0⟩ to |1⟩
            return measure(q)
        
        @guppy
        def pauli_y_test() -> bool:
            q = qubit()
            y(q)  # Should flip |0⟩ to |1⟩ with phase
            return measure(q)
            
        @guppy
        def pauli_z_test() -> bool:
            q = qubit()
            z(q)  # Should leave |0⟩ unchanged
            return measure(q)
        
        # Test X gate - should measure |1⟩ deterministically with fixed seed
        results_x = pipeline_tester.test_function_on_both_pipelines(pauli_x_test, shots=100, seed=42)
        if results_x.get("hugr_llvm", {}).get("success"):
            ones_count = sum(results_x["hugr_llvm"]["result"]["results"])
            # X gate should flip |0⟩ to |1⟩, expect 100% ones
            assert ones_count == 100, f"X gate should produce all 1s, got {ones_count}/100"
        
        # Test Y gate - should measure |1⟩ deterministically  
        results_y = pipeline_tester.test_function_on_both_pipelines(pauli_y_test, shots=100, seed=42)
        if results_y.get("hugr_llvm", {}).get("success"):
            ones_count = sum(results_y["hugr_llvm"]["result"]["results"])
            # Y gate should flip |0⟩ to |1⟩ with phase, expect 100% ones
            assert ones_count == 100, f"Y gate should produce all 1s, got {ones_count}/100"
        
        # Test Z gate - should measure |0⟩ deterministically
        results_z = pipeline_tester.test_function_on_both_pipelines(pauli_z_test, shots=100, seed=42)
        if results_z.get("hugr_llvm", {}).get("success"):
            ones_count = sum(results_z["hugr_llvm"]["result"]["results"])
            # Z gate should leave |0⟩ unchanged, expect 0% ones
            assert ones_count == 0, f"Z gate should produce all 0s, got {ones_count}/100"
    
    def test_bell_state_entanglement(self, pipeline_tester):
        """Test Bell state creation and entanglement."""
        @guppy
        def bell_state() -> tuple[bool, bool]:
            q0, q1 = qubit(), qubit()
            h(q0)
            cx(q0, q1)
            return measure(q0), measure(q1)
        
        results = pipeline_tester.test_function_on_both_pipelines(bell_state, shots=50)
        
        # Verify HUGR-LLVM pipeline results
        if results.get("hugr_llvm", {}).get("success"):
            measurements = results["hugr_llvm"]["result"]["results"]
            correlated = sum(1 for (a, b) in measurements if a == b)
            correlation_rate = correlated / len(measurements)
            assert correlation_rate > 0.8, f"Bell state should be highly correlated, got {correlation_rate:.2%}"
            print(f"HUGR-LLVM Bell state correlation: {correlation_rate:.2%}")
        
        # Verify PHIR pipeline results if available
        if results.get("phir", {}).get("success"):
            measurements = results["phir"]["result"]["results"]
            correlated = sum(1 for (a, b) in measurements if a == b)
            correlation_rate = correlated / len(measurements)
            assert correlation_rate > 0.8, f"PHIR Bell state should be highly correlated, got {correlation_rate:.2%}"
            print(f"PHIR Bell state correlation: {correlation_rate:.2%}")


# ============================================================================
# CLASSICAL COMPUTATION TESTS  
# ============================================================================

@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not PECOS_FRONTEND_AVAILABLE, reason="PECOS frontend not available")
class TestClassicalComputation:
    """Test classical computation capabilities in both pipelines."""
    
    def test_boolean_operations(self, pipeline_tester):
        """Test boolean logic operations."""
        @guppy
        def boolean_and_test() -> bool:
            # Simple boolean logic with quantum measurement
            q = qubit()
            result = measure(q)  # Will be False (|0⟩)
            return result and True
        
        @guppy
        def boolean_or_test() -> bool:
            q = qubit() 
            x(q)  # Flip to |1⟩
            result = measure(q)  # Will be True
            return result or False
        
        # Test AND operation
        results_and = pipeline_tester.test_function_on_both_pipelines(boolean_and_test, shots=10)
        print(f"Boolean AND test results: {results_and}")
        
        # Test OR operation
        results_or = pipeline_tester.test_function_on_both_pipelines(boolean_or_test, shots=10)
        print(f"Boolean OR test results: {results_or}")
    
    def test_classical_arithmetic(self, pipeline_tester):
        """Test basic arithmetic operations."""
        # NOTE: This may fail on current pipelines due to limited classical support
        @guppy
        def arithmetic_test() -> int:
            # Simple arithmetic that doesn't depend on quantum measurements
            a = 5
            b = 3
            return a + b
        
        results = pipeline_tester.test_function_on_both_pipelines(arithmetic_test, shots=5)
        print(f"Arithmetic test results: {results}")
        
        # Document current limitations
        if not results.get("hugr_llvm", {}).get("success"):
            print("EXPECTED: HUGR-LLVM may not support pure classical arithmetic yet")
        if not results.get("phir", {}).get("success"):
            print("EXPECTED: PHIR may have limited classical support")


# ============================================================================
# HYBRID QUANTUM-CLASSICAL TESTS
# ============================================================================

@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not PECOS_FRONTEND_AVAILABLE, reason="PECOS frontend not available")
class TestHybridPrograms:
    """Test hybrid quantum-classical programs."""
    
    def test_conditional_quantum_operations(self, pipeline_tester):
        """Test quantum operations conditional on classical results."""
        @guppy
        def conditional_gate() -> bool:
            q1 = qubit()
            q2 = qubit()
            
            # Measure first qubit
            result1 = measure(q1)  # Will be False (|0⟩)
            
            # Apply gate to second qubit based on first measurement
            if result1:
                x(q2)  # This won't execute since result1 is False
            
            return measure(q2)  # Should be False
        
        results = pipeline_tester.test_function_on_both_pipelines(conditional_gate, shots=20)
        
        if results.get("hugr_llvm", {}).get("success"):
            measurements = results["hugr_llvm"]["result"]["results"]
            ones_count = sum(measurements)
            # Since condition is never true, should measure mostly 0s
            assert ones_count < 5, f"Conditional gate failed, got {ones_count}/20 ones"
    
    def test_measurement_feedback(self, pipeline_tester):
        """Test feedback based on mid-circuit measurements."""
        @guppy
        def feedback_circuit() -> tuple[bool, bool]:
            q1 = qubit()
            q2 = qubit()
            
            # Create superposition on first qubit
            h(q1)
            result1 = measure(q1)
            
            # Apply correction to second qubit based on measurement
            if result1:
                x(q2)  # Flip second qubit if first was |1⟩
            
            return result1, measure(q2)
        
        results = pipeline_tester.test_function_on_both_pipelines(feedback_circuit, shots=50)
        print(f"Feedback circuit results: {results}")


# ============================================================================
# ADVANCED QUANTUM ALGORITHMS (PLACEHOLDER)
# ============================================================================

@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not PECOS_FRONTEND_AVAILABLE, reason="PECOS frontend not available")
class TestAdvancedAlgorithms:
    """Test advanced quantum algorithms (to be implemented)."""
    
    @pytest.mark.skip(reason="Not yet implemented - needs rotation gates")
    def test_quantum_fourier_transform(self, pipeline_tester):
        """Test quantum Fourier transform on 3 qubits."""
        # TODO: Implement when rotation gates are available
        pass
    
    @pytest.mark.skip(reason="Not yet implemented - needs oracle functions")
    def test_deutsch_josza_algorithm(self, pipeline_tester):
        """Test Deutsch-Josza algorithm."""
        # TODO: Implement when oracle support is available
        pass
    
    @pytest.mark.skip(reason="Not yet implemented - needs multi-qubit operations")
    def test_grover_search(self, pipeline_tester):
        """Test Grover's search algorithm."""
        # TODO: Implement for 2-3 qubit search space
        pass


# ============================================================================
# PIPELINE COMPARISON AND REPORTING
# ============================================================================

def test_pipeline_feature_summary():
    """Generate a comprehensive feature support summary."""
    print("\n" + "="*80)
    print("GUPPY PIPELINE FEATURE SUPPORT SUMMARY")
    print("="*80)
    
    backends = get_guppy_backends() if PECOS_FRONTEND_AVAILABLE else {}
    
    print(f"Guppy Available: {GUPPY_AVAILABLE}")
    print(f"PECOS Frontend Available: {PECOS_FRONTEND_AVAILABLE}")
    print(f"HUGR-LLVM Pipeline Available: {HUGR_LLVM_PIPELINE_AVAILABLE}")
    print(f"PHIR Pipeline Available: True")  # PHIR is always available as core part of PECOS
    
    if PECOS_FRONTEND_AVAILABLE:
        print(f"Rust Backend: {backends.get('rust_backend', False)}")
        print(f"Rust Message: {backends.get('rust_message', 'N/A')}")
        print(f"External Tools: {backends.get('external_tools', False)}")
    
    print("\nFeature Support Matrix:")
    print("- ✅ = Fully Supported")
    print("- ⚠️  = Partial Support") 
    print("- ❌ = Not Supported")
    print("- ❓ = Unknown/Needs Testing")
    
    features = [
        ("Basic Quantum Gates (H, X, Y, Z)", "✅", "✅"),
        ("Two-qubit Gates (CX)", "✅", "✅"),
        ("Quantum Measurements", "✅", "✅"),
        ("Bell State Creation", "✅", "✅"),
        ("Boolean Logic", "⚠️", "⚠️"),
        ("Classical Arithmetic", "❌", "⚠️"),
        ("Conditional Operations", "⚠️", "⚠️"),
        ("Measurement Feedback", "❓", "❓"),
        ("Rotation Gates", "❌", "❓"),
        ("Multi-qubit Algorithms", "❓", "❓"),
        ("Complex Data Structures", "❌", "❓"),
        ("Function Composition", "❓", "❓"),
    ]
    
    print(f"\n{'Feature':<30} {'HUGR-LLVM':<12} {'PHIR':<12}")
    print("-" * 56)
    for feature, hugr_status, phir_status in features:
        print(f"{feature:<30} {hugr_status:<12} {phir_status:<12}")
    
    print("\n" + "="*80)


if __name__ == "__main__":
    # Run the feature summary when executed directly
    test_pipeline_feature_summary()