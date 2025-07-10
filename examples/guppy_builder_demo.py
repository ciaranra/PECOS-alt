#!/usr/bin/env python3
"""Demonstrate the guppy_sim builder pattern and performance benefits.

This example shows how the builder pattern improves performance by
compiling once and running multiple times.
"""

import time
from guppylang import guppy
from guppylang.std.quantum import qubit, h, cx, measure

# Add quantum-pecos to path
import sys
sys.path.append("python/quantum-pecos/src")

from pecos.frontends import guppy_sim, run_guppy


@guppy
def bell_state() -> tuple[bool, bool]:
    """Create a Bell state."""
    q0, q1 = qubit(), qubit()
    h(q0)
    cx(q0, q1)
    return measure(q0), measure(q1)


@guppy
def ghz_3qubit() -> tuple[bool, bool, bool]:
    """Create a 3-qubit GHZ state."""
    q0, q1, q2 = qubit(), qubit(), qubit()
    h(q0)
    cx(q0, q1)
    cx(q1, q2)
    return measure(q0), measure(q1), measure(q2)


def demo_builder_pattern():
    """Demonstrate the builder pattern API."""
    print("=== Guppy Sim Builder Pattern Demo ===\n")
    
    # 1. Build once, run multiple times
    print("1. Building simulation once...")
    start = time.time()
    sim = guppy_sim(bell_state).seed(42).verbose(True).build()
    build_time = time.time() - start
    print(f"   Build time: {build_time:.4f}s\n")
    
    # Run multiple times without recompiling
    print("2. Running multiple shot counts without recompiling:")
    for shots in [100, 1000, 10000]:
        start = time.time()
        results = sim.run(shots)
        run_time = time.time() - start
        
        # Count correlations
        zeros = results["_result"].count(0)  # |00⟩
        threes = results["_result"].count(3)  # |11⟩
        
        print(f"   {shots:5d} shots: {run_time:.4f}s - |00⟩: {zeros}, |11⟩: {threes}")
    
    print("\n3. Configuration options:")
    # Binary string format
    results = guppy_sim(bell_state).binary_string_format().run(10)
    print(f"   Binary format: {results['_result']}")
    
    # Integer format (default)
    results = guppy_sim(bell_state).run(10)
    print(f"   Integer format: {results['_result']}")


def compare_performance():
    """Compare performance of builder pattern vs run_guppy."""
    print("\n=== Performance Comparison ===\n")
    
    shot_counts = [100, 100, 100]  # Run 3 times with same shots
    
    # Method 1: Using run_guppy (recompiles each time)
    print("1. Using run_guppy (recompiles each time):")
    total_time = 0
    for i, shots in enumerate(shot_counts):
        start = time.time()
        results = run_guppy(bell_state, shots=shots, seed=42)
        elapsed = time.time() - start
        total_time += elapsed
        print(f"   Run {i+1}: {elapsed:.4f}s")
    print(f"   Total: {total_time:.4f}s\n")
    
    # Method 2: Using builder pattern (compile once)
    print("2. Using guppy_sim builder (compile once):")
    start = time.time()
    sim = guppy_sim(bell_state).seed(42).build()
    build_time = time.time() - start
    print(f"   Build: {build_time:.4f}s")
    
    run_time = 0
    for i, shots in enumerate(shot_counts):
        start = time.time()
        results = sim.run(shots)
        elapsed = time.time() - start
        run_time += elapsed
        print(f"   Run {i+1}: {elapsed:.4f}s")
    
    total_builder_time = build_time + run_time
    print(f"   Total: {total_builder_time:.4f}s")
    
    speedup = total_time / total_builder_time
    print(f"\n   Speedup: {speedup:.2f}x faster with builder pattern!")


def demo_advanced_features():
    """Demonstrate advanced features."""
    print("\n=== Advanced Features ===\n")
    
    # 1. Complex circuit with configuration
    print("1. GHZ state with full configuration:")
    sim = (
        guppy_sim(ghz_3qubit)
        .seed(123)
        .workers(2)
        .verbose(False)
        .optimize(True)
        .build()
    )
    
    results = sim.run(1000)
    
    # Count GHZ correlations
    all_zeros = results["_result"].count(0)  # |000⟩ = 0
    all_ones = results["_result"].count(7)   # |111⟩ = 7
    
    print(f"   |000⟩: {all_zeros/10:.1%}, |111⟩: {all_ones/10:.1%}")
    
    # 2. Config dictionary
    print("\n2. Using config dictionary:")
    config = {
        "seed": 42,
        "workers": 4,
        "binary_string_format": True,
        "verbose": False,
    }
    
    results = guppy_sim(bell_state).config(config).run(20)
    print(f"   Results: {results['_result'][:10]}...")
    
    # 3. Direct run without explicit build
    print("\n3. Direct run (implicit build):")
    results = guppy_sim(bell_state).seed(99).run(50)
    print(f"   Got {len(results['_result'])} results")


if __name__ == "__main__":
    demo_builder_pattern()
    compare_performance() 
    demo_advanced_features()
    
    print("\n=== Demo Complete ===")
    
    # Cleanup temp files
    from pecos.frontends.guppy_sim_builder import GuppySimulation
    GuppySimulation.cleanup_all_temp_files()
    print("Cleaned up temporary files.")