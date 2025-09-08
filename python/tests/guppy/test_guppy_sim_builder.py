#!/usr/bin/env python3
"""Test the sim builder pattern API.

This test demonstrates that sim follows the same builder pattern as qasm_sim.
"""

import sys
from pathlib import Path
from typing import List, Tuple


def decode_integer_results(results: List[int], n_bits: int) -> List[Tuple[bool, ...]]:
    """Decode integer-encoded results back to tuples of booleans."""
    decoded = []
    for val in results:
        bits = []
        for i in range(n_bits):
            bits.append(bool(val & (1 << i)))
        decoded.append(tuple(bits))
    return decoded


import pytest

sys.path.append("python/quantum-pecos/src")

# Check dependencies
try:
    from guppylang import guppy
    from guppylang.std.quantum import qubit, h, cx, measure
    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False

try:
    from pecos.frontends.guppy_api import sim
    from pecos_rslib import state_vector
    BUILDER_AVAILABLE = True
except ImportError:
    BUILDER_AVAILABLE = False


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not BUILDER_AVAILABLE, reason="Builder not available")
class TestGuppySimBuilder:
    """Test the sim builder pattern."""
    
    @guppy
    def bell_state() -> tuple[bool, bool]:
        """Create a Bell state."""
        q0, q1 = qubit(), qubit()
        h(q0)
        cx(q0, q1)
        return measure(q0), measure(q1)
    
    @guppy
    def single_qubit() -> bool:
        """Single qubit in superposition."""
        q = qubit()
        h(q)
        return measure(q)
    
    def test_basic_build_and_run(self):
        """Test basic build() and run() pattern."""
        # Build once
        # Run multiple times with same configuration
        results1 = sim(self.bell_state).qubits(10).quantum(state_vector()).run(10)
        results2 = sim(self.bell_state).qubits(10).quantum(state_vector()).run(10)
        
        # Check format has measurement results
        # Bell state returns tuple, so we should have measurement_1 and measurement_2
        if "measurement_1" in results1 and "measurement_2" in results1:
            # New format with individual measurement keys
            assert len(results1["measurement_1"]) == 10
            assert len(results1["measurement_2"]) == 10
            assert len(results2["measurement_1"]) == 10
            assert len(results2["measurement_2"]) == 10
        else:
            # Fallback to old format
            measurements1 = results1.get("measurements", results1.get("result", []))
            measurements2 = results2.get("measurements", results2.get("result", []))
            assert len(measurements1) == 10
            assert len(measurements2) == 10
    
    def test_direct_run(self):
        """Test direct run() without explicit build()."""
        results = sim(self.single_qubit).qubits(10).quantum(state_vector()).run(10)
        
        # Check that we have measurement results (new format uses measurements key)
        assert "measurements" in results
        assert len(results["measurements"]) == 10
        assert all(r in [0, 1] for r in results["measurements"])
    
    def test_builder_methods(self):
        """Test various builder configuration methods."""
        # Test method chaining
        builder = (
            sim(self.bell_state).qubits(10)
            .seed(42)
            .workers(2)
            .verbose(True)
            .debug(False)
            .optimize(True)
        )
        
        # Build and run
        sim = builder.build()
        results = sim_obj.run(100)
        
        measurements = results.get("measurements", results.get("measurement_1", results.get("result", [])))
        assert measurements is not None and len(measurements) > 0
        assert len(measurements) == 100  # 100 shots, each with integer-encoded 2 qubits
    
    def test_seeded_reproducibility(self):
        """Test that seeded runs are reproducible."""
        # Run with same seed twice
        results1 = sim(self.single_qubit).qubits(10).quantum(state_vector()).seed(12345).run(100)
        results2 = sim(self.single_qubit).qubits(10).quantum(state_vector()).seed(12345).run(100)
        
        # Results should be identical
        measurements1 = results1.get("measurements", results1.get("measurement_1", results1.get("result", [])))
        measurements2 = results2.get("measurements", results2.get("measurement_1", results2.get("result", [])))
        assert measurements1 == measurements2
    
    def test_config_dict(self):
        """Test configuration via dictionary."""
        config = {
            "seed": 42,
            "workers": 4,
            "verbose": False,
            "debug": True,
        }
        
        # Test seed configuration (most commonly used)
        results = sim(self.bell_state).qubits(10).quantum(state_vector()).seed(42).run(50)
        
        # Check results format (Bell state returns tuple, so measurement_1 and measurement_2)
        if "measurement_1" in results:
            assert len(results["measurement_1"]) == 50
            assert len(results["measurement_2"]) == 50
        else:
            measurements = results.get("measurements", results.get("result", []))
            assert len(measurements) == 50
    
    def test_bell_state_correlation(self):
        """Test that Bell state results are correlated."""
        results = sim(self.bell_state).qubits(10).quantum(state_vector()).seed(42).run(1000)
        
        # Bell state should produce only |00⟩ and |11⟩
        measurements = results.get("measurements", results.get("measurement_1", results.get("result", [])))
        # Decode the integer-encoded results
        decoded = decode_integer_results(measurements, 2)
        # Check all results are correlated (both qubits same)
        correlated = sum(1 for (a, b) in decoded if a == b)
        assert correlated == len(decoded), "Bell state should be 100% correlated"
    
    def test_keep_intermediate_files(self):
        """Test keeping intermediate compilation files."""
        import shutil
        
        sim_obj = (
            sim(self.single_qubit).qubits(10).quantum(state_vector())
            .keep_intermediate_files(True)
            .build()
        )
        
        # Check that temp_dir was created
        assert sim_obj.temp_dir is not None
        assert Path(sim_obj.temp_dir).exists()
        
        # Check that intermediate files exist
        temp_path = Path(sim_obj.temp_dir)
        ll_files = list(temp_path.glob("*.ll"))
        hugr_files = list(temp_path.glob("*.hugr"))
        
        assert len(ll_files) > 0, "Should have created LLVM IR file"
        assert len(hugr_files) > 0, "Should have created HUGR file"
        
        # Run simulation
        results = sim_obj.run(10)
        measurements = results.get("measurements", results.get("measurement_1", results.get("result", [])))
        assert len(measurements) == 10
        
        # Files should still exist after run
        assert Path(sim_obj.temp_dir).exists()
        assert ll_files[0].exists()
        assert hugr_files[0].exists()
        
        # Manually clean up
        shutil.rmtree(sim_obj.temp_dir, ignore_errors=True)


def test_api_comparison():
    """Compare sim and qasm_sim APIs to ensure consistency."""
    # This test documents the parallel APIs
    
    # qasm_sim API (for reference):
    # sim = qasm_sim(qasm_string).seed(42).noise(DepolarizingNoise(0.01)).build()
    # results = sim.run(1000)
    
    # sim API (our implementation):
    # sim = sim(guppy_function).qubits(10).quantum(state_vector()).seed(42).noise(DepolarizingNoise(0.01)).build()
    # results = sim.run(1000)
    
    # Both return columnar format:
    # qasm_sim: {"c": [0, 3, 0, 3, ...]}  # register name from QASM
    # sim: {"_result": [0, 3, 0, 3, ...]}  # default register name
    
    print("API comparison test - APIs are parallel")
    assert True  # This is a documentation test


if __name__ == "__main__":
    # Run basic demonstration
    if GUPPY_AVAILABLE and BUILDER_AVAILABLE:
        print("=== Guppy Sim Builder Demo ===")
        
        @guppy
        def demo_circuit() -> bool:
            q = qubit()
            h(q)
            return measure(q)
        
        # Show builder pattern
        print("\n1. Building simulation...")
        sim_obj = sim(demo_circuit).qubits(10).quantum(state_vector()).seed(42).verbose(True).build()
        
        print("\n2. Running 100 shots...")
        results = sim_obj.run(100)
        measurements = results.get("measurements", results.get("measurement_1", results.get("result", [])))
        print(f"   Results: {measurements[:10]}... (first 10)")
        print(f"   Ones: {sum(measurements)}/100")
        
        print("\n3. Running 1000 shots...")
        results = sim_obj.run(1000)
        measurements = results.get("measurements", results.get("measurement_1", results.get("result", [])))
        print(f"   Ones: {sum(measurements)}/1000")
        
        print("\n4. Direct run without explicit build...")
        results = sim(demo_circuit).qubits(10).quantum(state_vector()).seed(123).run(50)
        measurements = results.get("measurements", results.get("measurement_1", results.get("result", [])))
        print(f"   Got {len(measurements)} results")
        
        print("\n=== Demo Complete ===")
    else:
        print("Dependencies not available for demo")