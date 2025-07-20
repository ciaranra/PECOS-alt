"""Examples demonstrating the structured configuration approach for PECOS QASM simulations.

This file shows how to use the Rust-native GeneralNoiseModelBuilder with fluent method chaining
to configure quantum simulations with better type safety and IDE support compared to
the legacy dictionary-based approach.
"""

from pecos_rslib.qasm_sim import (
    qasm_sim,
    QuantumEngine,
    GeneralNoiseModelBuilder,  # Rust-native builder
    DepolarizingNoise,
    DepolarizingCustomNoise,
    BiasedDepolarizingNoise,
    GeneralNoise,
)
from collections import Counter


def example_basic_noise_builder():
    """Example 1: Basic usage of Rust GeneralNoiseModelBuilder."""
    print("\n=== Example 1: Direct Rust GeneralNoiseModelBuilder ===")

    # Create a simple Bell state circuit
    qasm = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[2];
    creg c[2];
    h q[0];
    cx q[0], q[1];
    measure q -> c;
    """

    # Create and configure Rust-native builder with fluent chaining
    builder = (
        GeneralNoiseModelBuilder()
        .with_seed(42)
        .with_p1_probability(0.001)  # Single-qubit gate error
        .with_p2_probability(0.01)  # Two-qubit gate error
        .with_meas_0_probability(0.002)  # 0->1 measurement flip
        .with_meas_1_probability(0.002)
    )  # 1->0 measurement flip

    # Use builder directly with .noise() - just like Rust API!
    results = qasm_sim(qasm).noise(builder).run(1000)

    # Analyze results
    counts = Counter(results["c"])
    print(f"Bell state measurement results: {dict(counts)}")
    print("Expected: mostly 0 (|00>) and 3 (|11>) with some errors")
    print("Note: Using Rust-native builder for maximum performance")


def example_advanced_noise_builder():
    """Example 2: Advanced GeneralNoiseModelBuilder with detailed noise configuration."""
    print("\n=== Example 2: Advanced GeneralNoiseModelBuilder ===")

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

    # Build complex noise model
    noise = (
        GeneralNoiseModelBuilder()
        # Global parameters
        .with_seed(42)
        .with_scale(1.2)  # Scale all error rates by 1.2
        .with_noiseless_gate("H")  # H gates have no noise
        # Single-qubit gate noise with Pauli distribution
        .with_average_p1_probability(0.001)  # Average error (converted to total)
        .with_p1_pauli_model(
            {
                "X": 0.5,  # 50% X errors
                "Y": 0.3,  # 30% Y errors
                "Z": 0.2,  # 20% Z errors
            }
        )
        # Two-qubit gate noise
        .with_average_p2_probability(0.008)  # Average error (converted to total)
        # Preparation and measurement noise
        .with_prep_probability(0.001)
        .with_meas_0_probability(0.002)
        .with_meas_1_probability(0.003)
    )  # Asymmetric measurement error

    results = qasm_sim(qasm).noise(noise).run(1000)
    counts = Counter(results["c"])
    print(f"GHZ-like state results: {dict(counts)}")
    print("Expected: mostly 0 (|000>) and 7 (|111>) with errors")


def example_direct_configuration():
    """Example 3: Using direct method chaining for complete simulation setup."""
    print("\n=== Example 3: Direct Method Chaining ===")

    qasm = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[4];
    creg c[4];
    h q[0];
    h q[2];
    cx q[0], q[1];
    cx q[2], q[3];
    measure q -> c;
    """

    # Create noise using builder
    noise = (
        GeneralNoiseModelBuilder().with_p1_probability(0.001).with_p2_probability(0.01)
    )

    # Configure entire simulation with method chaining
    sim = (
        qasm_sim(qasm)
        .seed(42)
        .auto_workers()  # Automatically use all CPU cores
        .noise(noise)
        .quantum_engine(QuantumEngine.StateVector)
        .with_binary_string_format()  # Output as binary strings
        .build()
    )

    results_100 = sim.run(100)
    results_1000 = sim.run(1000)

    print("First run (100 shots):")
    print(f"  Sample results: {results_100['c'][:5]}")
    print("  Format: binary strings of length 4")

    print("Second run (1000 shots):")
    counts = Counter(results_1000["c"])
    print(f"  Most common states: {counts.most_common(4)}")


def example_builder_vs_direct():
    """Example 4: Comparing Python builder vs GeneralNoise dataclass."""
    print("\n=== Example 4: Builder vs Direct Configuration ===")

    qasm = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[2];
    creg c[2];
    h q[0];
    cx q[0], q[1];
    measure q -> c;
    """

    # APPROACH 1: Using GeneralNoiseModelBuilder with method chaining
    print("Using GeneralNoiseModelBuilder with method chaining:")
    noise_via_builder = (
        GeneralNoiseModelBuilder()
        .with_p1_probability(0.001)
        .with_p2_probability(0.01)
        .with_meas_0_probability(0.002)
        .with_meas_1_probability(0.002)
        .with_noiseless_gate("H")
        .with_p1_pauli_model({"X": 0.5, "Y": 0.3, "Z": 0.2})
    )

    results_builder = (
        qasm_sim(qasm)
        .seed(42)
        .workers(4)
        .noise(noise_via_builder)
        .quantum_engine(QuantumEngine.StateVector)
        .run(100)
    )
    print(f"  Results type: {type(results_builder['c'][0])} (integers)")

    # APPROACH 2: Using GeneralNoise directly
    print("\nUsing GeneralNoise dataclass directly:")
    noise_direct = GeneralNoise(
        p1=0.001,
        p2=0.01,
        p_meas_0=0.002,
        p_meas_1=0.002,
        noiseless_gates=["H"],
        p1_pauli_model={"X": 0.5, "Y": 0.3, "Z": 0.2},
    )

    results_direct = (
        qasm_sim(qasm)
        .seed(42)
        .workers(4)
        .noise(noise_direct)
        .quantum_engine(QuantumEngine.StateVector)
        .run(100)
    )
    print(f"  Results type: {type(results_direct['c'][0])} (integers)")
    print(f"  Results match: {results_builder['c'] == results_direct['c']}")


def example_different_noise_models():
    """Example 5: Using different built-in noise models."""
    print("\n=== Example 5: Different Noise Models ===")

    qasm = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[1];
    creg c[1];
    x q[0];
    measure q[0] -> c[0];
    """

    # Test different noise models
    noise_models = [
        ("No noise", None),
        ("Depolarizing", DepolarizingNoise(p=0.1)),
        (
            "Custom depolarizing",
            DepolarizingCustomNoise(p_prep=0.01, p_meas=0.05, p1=0.02, p2=0.03),
        ),
        ("Biased depolarizing", BiasedDepolarizingNoise(p=0.1)),
        (
            "General (builder)",
            GeneralNoiseModelBuilder().with_meas_1_probability(0.1),
        ),  # 10% chance to flip 1->0
    ]

    for name, noise in noise_models:
        results = qasm_sim(qasm).seed(42).noise(noise).run(1000)
        errors = sum(1 for val in results["c"] if val == 0)
        print(f"{name:20} - Errors: {errors}/1000 ({errors/10:.1f}%)")


def example_ion_trap_noise():
    """Example 6: Realistic ion trap noise model."""
    print("\n=== Example 6: Ion Trap Noise Model ===")

    qasm = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[5];
    creg c[5];

    // Create W state
    x q[0];
    h q[1];
    cx q[1], q[2];
    cx q[2], q[3];
    cx q[3], q[4];
    cx q[0], q[1];
    h q[1];

    measure q -> c;
    """

    # Realistic ion trap noise parameters
    noise = (
        GeneralNoiseModelBuilder()
        .with_seed(42)
        # Ion trap typical parameters
        .with_prep_probability(0.001)  # State prep error
        # Single-qubit gates (typically very good)
        .with_p1_probability(0.0001)
        # Two-qubit gates (main error source)
        .with_p2_probability(0.003)
        # Measurement (asymmetric for ions)
        .with_meas_0_probability(0.001)  # Dark state error
        .with_meas_1_probability(0.005)
    )  # Bright state error

    results = qasm_sim(qasm).noise(noise).run(1000)
    counts = Counter(results["c"])
    print("W-state preparation results (top 5):")
    for state, count in counts.most_common(5):
        binary = format(state, "05b")
        print(f"  |{binary}> : {count}")


def main():
    """Run all examples."""
    print("PECOS Structured Configuration Examples")
    print("=" * 50)

    example_basic_noise_builder()
    example_advanced_noise_builder()
    example_direct_configuration()
    example_builder_vs_direct()
    example_different_noise_models()
    example_ion_trap_noise()

    print("\n" + "=" * 50)
    print("Examples completed!")


if __name__ == "__main__":
    main()
