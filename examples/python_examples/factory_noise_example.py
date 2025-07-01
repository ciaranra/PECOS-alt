#!/usr/bin/env python3
"""GeneralNoiseFactory Example: Configurable Quantum Noise Models.

This example demonstrates how to use the GeneralNoiseFactory to create
quantum noise models from dictionary/JSON configurations.
"""

from collections import Counter

from pecos.rslib import GeneralNoiseFactory, create_noise_from_json, qasm_sim


def basic_factory_example() -> None:
    """Basic usage of GeneralNoiseFactory with default mappings."""
    print("\n=== Basic Factory Example ===")

    # QASM circuit: Bell state preparation
    qasm = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[2];
    creg c[2];
    h q[0];
    cx q[0], q[1];
    measure q -> c;
    """

    # Create factory with default mappings
    factory = GeneralNoiseFactory()

    # Define noise configuration
    config = {
        "seed": 42,
        "p1": 0.001,  # Single-qubit gate error
        "p2": 0.01,  # Two-qubit gate error
        "p_meas_0": 0.002,  # Measurement 0->1 flip
        "p_meas_1": 0.002,  # Measurement 1->0 flip
    }

    # Create noise model
    noise = factory.create_from_dict(config)

    # Run simulation
    results = qasm_sim(qasm).noise(noise).run(1000)

    # Analyze results
    counts = Counter(results["c"])
    print(f"Bell state results: {dict(counts)}")
    print("Expected: mostly 0 (|00>) and 3 (|11>) with some errors")


def custom_terminology_example() -> None:
    """Create a factory with custom parameter names."""
    print("\n=== Custom Terminology Example ===")

    # Create empty factory
    factory = GeneralNoiseFactory.empty()

    # Add mappings with custom terminology
    factory.add_mapping(
        "init_error",
        "with_prep_probability",
        float,
        "Initialization error rate",
    )
    factory.add_mapping(
        "single_gate_infidelity",
        "with_p1_probability",
        float,
        "Single-qubit gate infidelity",
    )
    factory.add_mapping(
        "entangling_gate_infidelity",
        "with_p2_probability",
        float,
        "Two-qubit entangling gate infidelity",
    )
    factory.add_mapping(
        "readout_error_0to1",
        "with_meas_0_probability",
        float,
        "Readout error P(1|0)",
    )
    factory.add_mapping(
        "readout_error_1to0",
        "with_meas_1_probability",
        float,
        "Readout error P(0|1)",
    )
    factory.add_mapping("random_seed", "with_seed", int, "Random number generator seed")

    # Show the custom mappings
    print("Custom parameter mappings:")
    factory.show_mappings(show_descriptions=False)

    # Use custom configuration
    config = {
        "random_seed": 42,
        "init_error": 0.0005,
        "single_gate_infidelity": 0.001,
        "entangling_gate_infidelity": 0.01,
        "readout_error_0to1": 0.002,
        "readout_error_1to0": 0.003,
    }

    factory.create_from_dict(config)
    print("\nNoise model created with custom parameters")


def json_configuration_example() -> None:
    """Load noise configuration from JSON."""
    print("\n=== JSON Configuration Example ===")

    # JSON configuration string (could be loaded from file)
    json_config = """
    {
        "seed": 42,
        "scale": 1.5,
        "p1": 0.001,
        "p2": 0.01,
        "noiseless_gates": ["H", "S", "T"],
        "p1_pauli": {
            "X": 0.6,
            "Y": 0.2,
            "Z": 0.2
        },
        "p_meas_0": 0.002,
        "p_meas_1": 0.005
    }
    """

    # Create noise directly from JSON
    create_noise_from_json(json_config)

    print("Noise model created from JSON with:")
    print("- 1.5x scaling of all error rates")
    print("- H, S, T gates are noiseless")
    print("- Custom Pauli error distribution for single-qubit gates")
    print("- Asymmetric measurement errors")


def validation_example() -> None:
    """Demonstrate configuration validation and error handling."""
    print("\n=== Validation Example ===")

    factory = GeneralNoiseFactory()

    # Configuration with errors
    bad_config = {
        "p1": "not_a_number",  # Type error
        "unknown_param": 0.001,  # Unknown key
        "p2": 0.01,  # Valid
        "seed": 42.5,  # Will be converted to int
    }

    # Validate configuration
    errors = factory.validate_config(bad_config)
    if errors:
        print("Validation errors found:")
        for key, error in errors.items():
            print(f"  {key}: {error}")

    # Demonstrate strict vs non-strict mode
    print("\nStrict mode behavior:")
    try:
        factory.create_from_dict(bad_config, strict=True)
    except ValueError as e:
        print(f"  Error (expected): {e}")

    print("\nNon-strict mode behavior:")
    # Non-strict mode ignores unknown keys
    factory.create_from_dict(
        {"p1": 0.001, "p2": 0.01, "unknown": 123},
        strict=False,
    )
    print("  Noise model created (unknown keys ignored)")


def cleanup_aliases_example() -> None:
    """Remove confusing aliases to simplify the API."""
    print("\n=== Cleanup Aliases Example ===")

    factory = GeneralNoiseFactory()

    # Show initial key count
    print(f"Initial mappings: {len(factory.mappings)} keys")

    # Remove aliases to keep only primary keys
    removed = [
        alias
        for alias in ["prep", "p1_total", "p2_total", "p_meas_0", "p_meas_1"]
        if factory.remove_mapping(alias)
    ]

    print(f"Removed {len(removed)} aliases: {removed}")
    print(f"Remaining mappings: {len(factory.mappings)} keys")

    # Now only primary keys work
    config = {
        "p_prep": 0.0005,  # Primary key
        "p1": 0.001,  # Primary key
        "p2": 0.01,  # Primary key
        "p_meas_0": 0.002,  # Primary key
        "p_meas_1": 0.003,  # Primary key
    }

    factory.create_from_dict(config)
    print("Noise model created with primary keys only")


def factory_with_defaults_example() -> None:
    """Set factory-wide default values."""
    print("\n=== Factory Defaults Example ===")

    factory = GeneralNoiseFactory()

    # Set common defaults
    factory.set_default("seed", 42)
    factory.set_default("p1", 0.001)
    factory.set_default("p2", 0.01)
    factory.set_default("p_meas_0", 0.002)
    factory.set_default("p_meas_1", 0.002)

    # Empty config uses all defaults
    factory.create_from_dict({})
    print("Created noise model with all defaults")

    # Override specific values
    factory.create_from_dict(
        {
            "p2": 0.005,  # Override two-qubit error
            "scale": 0.5,  # Scale down all errors by 50%
        },
    )
    print("Created noise model with partial overrides")


def advanced_noise_example() -> None:
    """Complex noise configuration with many features."""
    print("\n=== Advanced Noise Example ===")

    # GHZ state preparation circuit
    qasm = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[4];
    creg c[4];
    h q[0];
    cx q[0], q[1];
    cx q[1], q[2];
    cx q[2], q[3];
    measure q -> c;
    """

    factory = GeneralNoiseFactory()

    config = {
        # Global settings
        "seed": 42,
        "scale": 1.2,  # Scale all errors by 20%
        # Make specific gates noiseless
        "noiseless_gates": ["H"],
        # State preparation
        "p_prep": 0.0005,
        # Single-qubit gates with Pauli distribution
        "p1_average": 0.001,
        "p1_pauli": {
            "X": 0.5,  # 50% X errors
            "Y": 0.3,  # 30% Y errors
            "Z": 0.2,  # 20% Z errors
        },
        # Two-qubit gates with higher error
        "p2_average": 0.008,
        # Asymmetric measurement errors
        "p_meas_0": 0.002,  # Lower 0->1 flip
        "p_meas_1": 0.005,  # Higher 1->0 flip
    }

    noise = factory.create_from_dict(config)
    results = qasm_sim(qasm).noise(noise).run(1000)

    counts = Counter(results["c"])
    print("GHZ state results (top 5):")
    for state, count in counts.most_common(5):
        binary = format(state, "04b")
        print(f"  |{binary}>: {count}")
    print("Expected: mostly |0000> and |1111> with errors due to noise")


def main() -> None:
    """Run all examples."""
    print("GeneralNoiseFactory Examples")
    print("=" * 50)

    basic_factory_example()
    custom_terminology_example()
    json_configuration_example()
    validation_example()
    cleanup_aliases_example()
    factory_with_defaults_example()
    advanced_noise_example()

    print("\n" + "=" * 50)
    print("Examples completed!")


if __name__ == "__main__":
    main()
