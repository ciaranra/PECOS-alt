#!/usr/bin/env python3
"""QASM Simulation API Example

This example demonstrates the PECOS QASM simulation API with various
noise models and quantum engines.
"""

from collections import Counter
from pecos_rslib.qasm_sim import (
    run_qasm,
    qasm_sim,
    QuantumEngine,
    DepolarizingNoise,
    DepolarizingCustomNoise,
    BiasedDepolarizingNoise,
)


def example_bell_state():
    """Example: Create and measure a Bell state."""
    print("\n=== Bell State Example ===")

    qasm = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[2];
    creg c[2];
    h q[0];
    cx q[0], q[1];
    measure q -> c;
    """

    # Run without noise
    results = run_qasm(qasm, shots=1000)
    counts = Counter(results["c"])

    print("Bell state measurements (no noise):")
    for outcome, count in sorted(counts.items()):
        print(f"  |{outcome:02b}⟩: {count} times")

    # Run with depolarizing noise
    results_noisy = run_qasm(
        qasm, shots=1000, noise_model=DepolarizingNoise(p=0.02), seed=42
    )
    counts_noisy = Counter(results_noisy["c"])

    print("\nBell state measurements (2% depolarizing noise):")
    for outcome, count in sorted(counts_noisy.items()):
        print(f"  |{outcome:02b}⟩: {count} times")


def example_ghz_state():
    """Example: Create and measure a 3-qubit GHZ state."""
    print("\n=== GHZ State Example ===")

    qasm = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[3];
    creg c[3];
    h q[0];
    cx q[0], q[1];
    cx q[1], q[2];
    measure q -> c;
    """

    # Run with custom depolarizing noise
    noise = DepolarizingCustomNoise(
        p_prep=0.001,  # Low preparation error
        p_meas=0.005,  # Moderate measurement error
        p1=0.001,  # Low single-qubit gate error
        p2=0.01,  # Higher two-qubit gate error
    )

    results = run_qasm(
        qasm,
        shots=1000,
        noise_model=noise,
        engine=QuantumEngine.SparseStabilizer,
        seed=42,
    )

    counts = Counter(results["c"])
    print("GHZ state measurements (custom noise):")
    for outcome, count in sorted(counts.items()):
        print(f"  |{outcome:03b}⟩: {count} times")


def example_biased_depolarizing():
    """Example: Demonstrate biased depolarizing noise."""
    print("\n=== Biased Depolarizing Example ===")

    qasm = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[2];
    creg c[2];
    x q[0];
    x q[1];
    measure q -> c;
    """

    # Perfect measurements
    results_ideal = run_qasm(qasm, shots=1000)
    ideal_counts = Counter(results_ideal["c"])

    # Biased depolarizing noise
    noise = BiasedDepolarizingNoise(
        p=0.1,  # 10% error probability
    )

    results_biased = run_qasm(qasm, shots=1000, noise_model=noise, seed=42)
    biased_counts = Counter(results_biased["c"])

    print("Preparing |11⟩ state:")
    print(f"  Ideal: {ideal_counts}")
    print(f"  Biased depolarizing: {biased_counts}")
    print("  (Notice the errors introduced by biased depolarizing noise)")


def example_quantum_engines():
    """Example: Compare different quantum engines."""
    print("\n=== Quantum Engine Comparison ===")

    # Circuit with non-Clifford gates
    qasm = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[2];
    creg c[2];
    h q[0];
    rz(0.5) q[0];
    cx q[0], q[1];
    measure q -> c;
    """

    # State vector engine (can handle arbitrary gates)
    try:
        results_sv = run_qasm(
            qasm, shots=100, engine=QuantumEngine.StateVector, seed=42
        )
        sv_counts = Counter(results_sv["c"])
        print(f"StateVector engine: {dict(sv_counts)}")
    except Exception as e:
        print(f"StateVector engine error: {e}")

    # Sparse stabilizer engine (efficient for Clifford circuits)
    # This will fail for non-Clifford gates like rz(0.5)
    try:
        results_stab = run_qasm(
            qasm, shots=100, engine=QuantumEngine.SparseStabilizer, seed=42
        )
        stab_counts = Counter(results_stab["c"])
        print(f"SparseStabilizer engine: {dict(stab_counts)}")
    except Exception:
        print(
            "SparseStabilizer engine error: Expected - cannot handle non-Clifford gates"
        )


def example_builder_pattern():
    """Example: Using the builder pattern for reusable simulations."""
    print("\n=== Builder Pattern Example ===")

    qasm = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[2];
    creg c[2];
    h q[0];
    cx q[0], q[1];
    measure q -> c;
    """

    # Build once, run multiple times with different shot counts
    sim = (
        qasm_sim(qasm)
        .seed(42)
        .noise(DepolarizingNoise(p=0.01))
        .quantum_engine(QuantumEngine.SparseStabilizer)
        .workers(4)
        .build()
    )

    print("Running same circuit with different shot counts:")
    for shots in [10, 100, 1000]:
        results = sim.run(shots)
        counts = Counter(results["c"])
        print(f"  {shots} shots: {dict(counts)}")

    # Or run directly without building
    results = qasm_sim(qasm).noise(BiasedDepolarizingNoise(p=0.005)).run(500)

    counts = Counter(results["c"])
    print(f"\nDirect run with biased depolarizing noise: {dict(counts)}")


def example_large_register():
    """Example: Handling large quantum registers (>64 qubits)."""
    print("\n=== Large Register Example ===")

    # Create a circuit with 70 qubits
    qasm = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[70];
    creg c[70];

    // Create a pattern
    x q[0];
    x q[10];
    x q[20];
    x q[30];
    x q[40];
    x q[50];
    x q[60];
    x q[69];

    measure q -> c;
    """

    results = run_qasm(qasm, shots=10)

    print("Large register measurements (70 qubits):")
    for i, value in enumerate(results["c"][:5]):  # Show first 5
        # Convert to binary string for large values
        binary = bin(value)[2:].zfill(70)
        set_bits = [i for i, bit in enumerate(reversed(binary)) if bit == "1"]
        print(f"  Shot {i}: bits {set_bits} are set")
    print(f"  ... ({len(results['c'])} total shots)")


def example_parallel_execution():
    """Example: Parallel execution with multiple workers."""
    print("\n=== Parallel Execution Example ===")

    qasm = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[5];
    creg c[5];

    // Random circuit
    h q[0];
    h q[1];
    h q[2];
    cx q[0], q[3];
    cx q[1], q[4];
    cx q[2], q[3];
    h q[3];
    h q[4];

    measure q -> c;
    """

    import time

    # Single worker
    start = time.time()
    run_qasm(
        qasm, shots=10000, noise_model=DepolarizingNoise(p=0.001), workers=1, seed=42
    )
    single_time = time.time() - start

    # Multiple workers
    start = time.time()
    run_qasm(
        qasm, shots=10000, noise_model=DepolarizingNoise(p=0.001), workers=4, seed=42
    )
    parallel_time = time.time() - start

    print("Execution time comparison (10,000 shots):")
    print(f"  Single worker: {single_time:.3f}s")
    print(f"  4 workers: {parallel_time:.3f}s")
    print(f"  Speedup: {single_time/parallel_time:.2f}x")

    # Note: Results may differ slightly between single and multi-worker runs
    # due to different random number generation patterns


if __name__ == "__main__":
    print("PECOS QASM Simulation API Examples")
    print("==================================")

    example_bell_state()
    example_ghz_state()
    example_biased_depolarizing()
    example_quantum_engines()
    example_builder_pattern()
    example_large_register()
    example_parallel_execution()

    print("\nAll examples completed successfully!")
