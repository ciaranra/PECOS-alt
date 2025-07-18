"""Examples demonstrating the GeneralNoiseFactory for dict/JSON-based configuration.

This shows how to create GeneralNoiseModelBuilder instances from dictionaries
or JSON configuration while maintaining the benefits of the builder pattern.
"""

import json
from pecos_rslib.qasm_sim import qasm_sim
from pecos_rslib.general_noise_factory import (
    GeneralNoiseFactory,
    create_noise_from_dict,
    create_noise_from_json,
    IonTrapNoiseFactory,
)


def example_basic_dict_config():
    """Example 1: Basic dictionary configuration."""
    print("\n=== Example 1: Basic Dictionary Configuration ===")

    # Define noise configuration as a dictionary
    noise_config = {
        "seed": 42,
        "p1": 0.001,  # Single-qubit gate error
        "p2": 0.01,  # Two-qubit gate error
        "p_meas_0": 0.002,  # 0->1 measurement flip
        "p_meas_1": 0.002,  # 1->0 measurement flip
        "scale": 1.2,  # Scale all errors by 1.2x
    }

    # Create noise model from dictionary
    noise = create_noise_from_dict(noise_config)

    # Use in simulation
    qasm = """
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[2];
    creg c[2];
    h q[0];
    cx q[0], q[1];
    measure q -> c;
    """

    results = qasm_sim(qasm).noise(noise).run(1000)
    print(f"Created noise model from dict: {noise_config}")
    print(f"Ran simulation, got {len(results['c'])} results")


def example_json_config():
    """Example 2: JSON configuration with validation."""
    print("\n=== Example 2: JSON Configuration ===")

    # JSON configuration string (could be loaded from file)
    json_config = """
    {
        "seed": 123,
        "p1_average": 0.0008,
        "p2_average": 0.008,
        "p1_pauli_model": {"X": 0.5, "Y": 0.3, "Z": 0.2},
        "noiseless_gates": ["H", "MEASURE"],
        "p_meas_0": 0.001,
        "p_meas_1": 0.003
    }
    """

    # Create noise model from JSON
    create_noise_from_json(json_config)

    print("Created noise model from JSON")
    print("Configuration included:")
    print("- Average gate errors (converted to total)")
    print("- Pauli error distribution")
    print("- Noiseless gates: H, MEASURE")
    print("- Asymmetric measurement errors")


def example_custom_factory():
    """Example 3: Custom factory with defaults and mappings."""
    print("\n=== Example 3: Custom Factory ===")

    # Create custom factory for superconducting qubits
    factory = GeneralNoiseFactory()

    # Set typical defaults for superconducting systems
    factory.set_default("p1", 0.0005)
    factory.set_default("p2", 0.005)
    factory.set_default("p_meas_0", 0.01)
    factory.set_default("p_meas_1", 0.01)

    # Add custom mapping for T1/T2 times
    def t1_to_emission_ratio(t1_us: float) -> float:
        """Convert T1 time in microseconds to emission ratio."""
        # Rough approximation: shorter T1 = more emission
        return min(1.0, 10.0 / t1_us)

    factory.add_mapping(
        "t1_time",
        "with_emission_scale",
        t1_to_emission_ratio,
        "T1 coherence time in microseconds",
    )

    # User only needs to specify deviations from defaults
    config = {
        "seed": 42,
        "t1_time": 50.0,  # 50 microsecond T1
        "p2": 0.008,  # Override default for two-qubit gates
    }

    factory.create_from_dict(config)
    print("Custom factory applied:")
    print("- Defaults: p1=0.0005, p2=0.005, p_meas=0.01")
    print("- User overrides: p2=0.008, T1=50μs")
    print("- T1 converted to emission scale")


def example_validation_and_errors():
    """Example 4: Configuration validation and error handling."""
    print("\n=== Example 4: Validation and Error Handling ===")

    factory = GeneralNoiseFactory()

    # Example 1: Invalid configuration with unknown keys
    bad_config = {
        "p1": 0.001,
        "p2": 0.01,
        "unknown_key": 123,  # This will cause error in strict mode
        "another_bad_key": "value",
    }

    # Validate before creating
    errors = factory.validate_config(bad_config)
    if errors:
        print("Validation errors found:")
        for key, error in errors.items():
            print(f"  {key}: {error}")

    # Try strict mode (will raise exception)
    try:
        factory.create_from_dict(bad_config, strict=True)
    except ValueError as e:
        print(f"\nStrict mode error: {e}")

    # Non-strict mode (ignores unknown keys)
    factory.create_from_dict(bad_config, strict=False)
    print("\nNon-strict mode: Unknown keys ignored, noise model created")

    # Example 2: Type validation
    bad_types = {
        "p1": "not_a_number",  # Should be float
        "seed": 3.14,  # Will be converted to int
    }

    errors = factory.validate_config(bad_types)
    print(f"\nType validation errors: {errors}")


def example_custom_key_mappings():
    """Example 5: Custom key mappings for domain-specific terminology."""
    print("\n=== Example 5: Custom Key Mappings ===")

    # Create factory with custom terminology
    factory = GeneralNoiseFactory()

    # Add custom mappings for your domain
    # Example: Map shorter/clearer names to builder methods
    factory.add_mapping(
        "p_sq",
        "with_average_p1_probability",
        float,
        "Single-qubit gate error probability",
    )
    factory.add_mapping(
        "p_tq", "with_average_p2_probability", float, "Two-qubit gate error probability"
    )
    factory.add_mapping(
        "readout_error",
        "with_meas_0_probability",
        float,
        "Symmetric readout error (applied to both 0->1 and 1->0)",
    )

    # You can also add mappings with custom converters
    def percent_to_probability(percent: float) -> float:
        """Convert percentage to probability (e.g., 0.1% -> 0.001)."""
        return percent / 100.0

    factory.add_mapping(
        "p_sq_percent",
        "with_average_p1_probability",
        percent_to_probability,
        "Single-qubit error as percentage",
    )

    # Example configuration using custom keys
    config = {
        "seed": 42,
        "p_sq": 0.001,  # Uses with_average_p1_probability
        "p_tq": 0.01,  # Uses with_average_p2_probability
        "p_sq_percent": 0.15,  # 0.15% -> 0.0015 probability
        "readout_error": 0.002,  # Applied to meas_0
    }

    # For asymmetric readout, we need to apply readout_error to both
    noise = factory.create_from_dict(config)
    # Manually add meas_1 since we mapped readout_error only to meas_0
    noise = noise.with_meas_1_probability(config["readout_error"])

    print("Custom mappings applied:")
    print("- p_sq → with_average_p1_probability")
    print("- p_tq → with_average_p2_probability")
    print("- p_sq_percent → with_average_p1_probability (with % conversion)")
    print("- readout_error → with_meas_0_probability")
    print("\nResulting config: p1_avg≈0.0015, p2_avg=0.01, readout=0.002")


def example_ion_trap_specialized():
    """Example 6: Specialized ion trap factory."""
    print("\n=== Example 6: Ion Trap Specialized Factory ===")

    # Use the specialized ion trap factory
    factory = IonTrapNoiseFactory()

    # Minimal configuration - relies on ion trap defaults
    config = {
        "seed": 42,
        "motional_heating": 2.0,  # Custom ion trap parameter
    }

    factory.create_from_dict(config)

    print("Ion trap factory applied:")
    print("- Ion trap specific defaults (p1=0.0001, p2=0.003, etc.)")
    print("- Motional heating converted to scale factor")
    print("- Asymmetric measurement errors (0.001/0.005)")


def example_available_keys():
    """Example 7: Discovering available configuration keys."""
    print("\n=== Example 7: Available Configuration Keys ===")

    factory = GeneralNoiseFactory()
    keys = factory.get_available_keys()

    print("Available configuration keys:")
    for key, description in sorted(keys.items()):
        print(f"  {key:15} - {description}")


def example_complex_configuration():
    """Example 8: Complex configuration with all features."""
    print("\n=== Example 8: Complex Configuration ===")

    # Complex configuration using many features
    config = {
        # Random seed
        "seed": 42,
        # Global scaling
        "scale": 1.5,
        "leakage_scale": 0.1,
        # Gate errors with Pauli models
        "p1_average": 0.001,
        "p1_pauli_model": {
            "X": 0.6,  # More bit flips
            "Y": 0.2,
            "Z": 0.2,  # Less phase flips
        },
        "p2_average": 0.008,
        "p2_pauli_model": {"IX": 0.25, "XI": 0.25, "XX": 0.5},
        # Noiseless gates
        "noiseless_gates": ["H", "S", "T"],
        # State prep and measurement
        "p_prep": 0.0005,
        "p_meas_0": 0.002,
        "p_meas_1": 0.003,
    }

    # Create and validate
    factory = GeneralNoiseFactory()
    errors = factory.validate_config(config)
    if not errors:
        print("Configuration is valid!")

    factory.create_from_dict(config)

    # Could save this config for reproducibility
    config_json = json.dumps(config, indent=2)
    print(f"\nConfiguration JSON (can be saved to file):\n{config_json}")


def main():
    """Run all examples."""
    print("GeneralNoiseFactory Examples")
    print("=" * 50)

    example_basic_dict_config()
    example_json_config()
    example_custom_factory()
    example_validation_and_errors()
    example_custom_key_mappings()
    example_ion_trap_specialized()
    example_available_keys()
    example_complex_configuration()

    print("\n" + "=" * 50)
    print("Examples completed!")


if __name__ == "__main__":
    main()
