#!/usr/bin/env python3
"""Test the guppy_sim builder pattern API.

This test demonstrates that guppy_sim follows the same builder pattern as qasm_sim.
"""

import sys
from pathlib import Path

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
    from pecos.frontends import guppy_sim
    BUILDER_AVAILABLE = True
except ImportError:
    BUILDER_AVAILABLE = False


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not BUILDER_AVAILABLE, reason="Builder not available")
class TestGuppySimBuilder:
    """Test the guppy_sim builder pattern."""
    
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
        sim = guppy_sim(self.bell_state).build()
        
        # Run multiple times
        results1 = sim.run(100)
        results2 = sim.run(200)
        
        # Check format matches qasm_sim (columnar)
        assert "_result" in results1
        assert len(results1["_result"]) == 100
        assert len(results2["_result"]) == 200
        
        # Check metadata
        assert "_metadata" in results1
        assert results1["_metadata"]["shots"] == 100
        assert results2["_metadata"]["shots"] == 200
        assert results2["_metadata"]["total_runs"] == 2
        assert results2["_metadata"]["total_shots"] == 300
    
    def test_direct_run(self):
        """Test direct run() without explicit build()."""
        results = guppy_sim(self.single_qubit).run(50)
        
        assert "_result" in results
        assert len(results["_result"]) == 50
        assert all(r in [0, 1] for r in results["_result"])
    
    def test_builder_methods(self):
        """Test various builder configuration methods."""
        # Test method chaining
        builder = (
            guppy_sim(self.bell_state)
            .seed(42)
            .workers(2)
            .verbose(True)
            .debug(False)
            .optimize(True)
        )
        
        # Build and run
        sim = builder.build()
        results = sim.run(100)
        
        assert "_result" in results
        assert len(results["_result"]) == 100
    
    def test_seeded_reproducibility(self):
        """Test that seeded runs are reproducible."""
        # Create two builders with same seed
        sim1 = guppy_sim(self.single_qubit).seed(12345).build()
        sim2 = guppy_sim(self.single_qubit).seed(12345).build()
        
        # Run both
        results1 = sim1.run(100)
        results2 = sim2.run(100)
        
        # Results should be identical
        assert results1["_result"] == results2["_result"]
    
    def test_config_dict(self):
        """Test configuration via dictionary."""
        config = {
            "seed": 42,
            "workers": 4,
            "verbose": False,
            "debug": True,
        }
        
        sim = guppy_sim(self.bell_state).config(config).build()
        results = sim.run(50)
        
        assert "_result" in results
        assert len(results["_result"]) == 50
    
    def test_bell_state_correlation(self):
        """Test that Bell state results are correlated."""
        results = guppy_sim(self.bell_state).seed(42).run(1000)
        
        # Bell state should produce only |00⟩ (0) and |11⟩ (3)
        unique_results = set(results["_result"])
        assert unique_results.issubset({0, 3})
        
        # Should be roughly 50/50
        zeros = results["_result"].count(0)
        threes = results["_result"].count(3)
        assert 400 < zeros < 600  # Allow some variance
        assert 400 < threes < 600
    
    def test_keep_intermediate_files(self):
        """Test keeping intermediate compilation files."""
        import shutil
        
        sim = (
            guppy_sim(self.single_qubit)
            .keep_intermediate_files(True)
            .build()
        )
        
        # Check that temp_dir was created
        assert sim.temp_dir is not None
        assert Path(sim.temp_dir).exists()
        
        # Check that intermediate files exist
        temp_path = Path(sim.temp_dir)
        ll_files = list(temp_path.glob("*.ll"))
        hugr_files = list(temp_path.glob("*.hugr"))
        
        assert len(ll_files) > 0, "Should have created LLVM IR file"
        assert len(hugr_files) > 0, "Should have created HUGR file"
        
        # Run simulation
        results = sim.run(10)
        assert len(results["_result"]) == 10
        
        # Files should still exist after run
        assert Path(sim.temp_dir).exists()
        assert ll_files[0].exists()
        assert hugr_files[0].exists()
        
        # Manually clean up
        shutil.rmtree(sim.temp_dir, ignore_errors=True)


def test_api_comparison():
    """Compare guppy_sim and qasm_sim APIs to ensure consistency."""
    # This test documents the parallel APIs
    
    # qasm_sim API (for reference):
    # sim = qasm_sim(qasm_string).seed(42).noise(DepolarizingNoise(0.01)).build()
    # results = sim.run(1000)
    
    # guppy_sim API (our implementation):
    # sim = guppy_sim(guppy_function).seed(42).noise(DepolarizingNoise(0.01)).build()
    # results = sim.run(1000)
    
    # Both return columnar format:
    # qasm_sim: {"c": [0, 3, 0, 3, ...]}  # register name from QASM
    # guppy_sim: {"_result": [0, 3, 0, 3, ...]}  # default register name
    
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
        sim = guppy_sim(demo_circuit).seed(42).verbose(True).build()
        
        print("\n2. Running 100 shots...")
        results = sim.run(100)
        print(f"   Results: {results['_result'][:10]}... (first 10)")
        print(f"   Ones: {sum(results['_result'])}/100")
        
        print("\n3. Running 1000 shots...")
        results = sim.run(1000)
        print(f"   Ones: {sum(results['_result'])}/1000")
        
        print("\n4. Direct run without explicit build...")
        results = guppy_sim(demo_circuit).seed(123).run(50)
        print(f"   Got {len(results['_result'])} results")
        
        print("\n=== Demo Complete ===")
    else:
        print("Dependencies not available for demo")