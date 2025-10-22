"""Tests for GeneralNoiseFactory."""

import json
import warnings
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    import pytest
from pecos_rslib import GeneralNoiseModelBuilder
from pecos_rslib.general_noise_factory import (
    GeneralNoiseFactory,
    IonTrapNoiseFactory,
    MethodMapping,
    create_noise_from_dict,
    create_noise_from_json,
)


class TestMethodMapping:
    """Test the MethodMapping class."""

    def test_basic_mapping(self) -> None:
        """Test basic method mapping without converter."""
        mapping = MethodMapping("with_seed", None, "Random seed")
        builder = GeneralNoiseModelBuilder()

        result = mapping.apply(builder, 42)
        assert isinstance(result, GeneralNoiseModelBuilder)

    def test_mapping_with_converter(self) -> None:
        """Test mapping with type converter."""
        mapping = MethodMapping("with_seed", int, "Random seed")
        builder = GeneralNoiseModelBuilder()

        # Should convert float to int
        result = mapping.apply(builder, 42.7)
        assert isinstance(result, GeneralNoiseModelBuilder)


class TestGeneralNoiseFactory:
    """Test the GeneralNoiseFactory class."""

    def test_basic_creation(self) -> None:
        """Test basic factory creation with simple config."""
        factory = GeneralNoiseFactory()
        config = {
            "seed": 42,
            "p1": 0.001,
            "p2": 0.01,
        }

        builder = factory.create_from_dict(config)
        assert isinstance(builder, GeneralNoiseModelBuilder)

    def test_all_standard_mappings(self) -> None:
        """Test that all standard mappings work correctly."""
        factory = GeneralNoiseFactory()
        config = {
            "seed": 123,
            "scale": 1.5,
            "leakage_scale": 0.2,
            "emission_scale": 0.3,
            "noiseless_gate": "H",
            "p_prep": 0.0005,
            "p1": 0.001,
            "p1_average": 0.0008,
            "p2": 0.01,
            "p2_average": 0.008,
            "p_meas_0": 0.002,
            "p_meas_1": 0.003,
        }

        builder = factory.create_from_dict(config)
        assert isinstance(builder, GeneralNoiseModelBuilder)

    def test_noiseless_gates_list(self) -> None:
        """Test handling of noiseless_gates list."""
        factory = GeneralNoiseFactory()
        config = {
            "seed": 42,
            "noiseless_gates": ["H", "X", "Y", "MEASURE"],
        }

        builder = factory.create_from_dict(config)
        assert isinstance(builder, GeneralNoiseModelBuilder)

    def test_pauli_models(self) -> None:
        """Test Pauli error model configurations."""
        factory = GeneralNoiseFactory()
        config = {
            "p1_pauli_model": {"X": 0.5, "Y": 0.3, "Z": 0.2},
            "p2_pauli_model": {"IX": 0.25, "XI": 0.25, "XX": 0.5},
        }

        builder = factory.create_from_dict(config)
        assert isinstance(builder, GeneralNoiseModelBuilder)

    def test_no_more_aliases(self) -> None:
        """Test that we removed confusing aliases."""
        factory = GeneralNoiseFactory()

        # These aliases should NOT work anymore
        with pytest.raises(ValueError, match="Unknown configuration keys"):
            factory.create_from_dict({"prep": 0.001}, strict=True)

        with pytest.raises(ValueError, match="Unknown configuration keys"):
            factory.create_from_dict({"p1_total": 0.001}, strict=True)

        with pytest.raises(ValueError, match="Unknown configuration keys"):
            factory.create_from_dict({"p2_total": 0.01}, strict=True)

        # But the primary keys should work
        builder = factory.create_from_dict({"p_prep": 0.001, "p1": 0.001, "p2": 0.01})
        assert isinstance(builder, GeneralNoiseModelBuilder)

    def test_strict_mode_unknown_keys(self) -> None:
        """Test that strict mode raises error for unknown keys."""
        factory = GeneralNoiseFactory()
        config = {
            "seed": 42,
            "unknown_key": 123,
            "another_bad": "value",
        }

        with pytest.raises(ValueError, match="Unknown configuration keys") as exc_info:
            factory.create_from_dict(config, strict=True)

        assert "Unknown configuration keys" in str(exc_info.value)
        assert "unknown_key" in str(exc_info.value)
        assert "another_bad" in str(exc_info.value)

    def test_non_strict_mode_ignores_unknown(self) -> None:
        """Test that non-strict mode ignores unknown keys."""
        factory = GeneralNoiseFactory()
        config = {
            "seed": 42,
            "p1": 0.001,
            "unknown_key": 123,
        }

        # Should not raise
        builder = factory.create_from_dict(config, strict=False)
        assert isinstance(builder, GeneralNoiseModelBuilder)

    def test_custom_mapping(self) -> None:
        """Test adding custom mappings."""
        factory = GeneralNoiseFactory()

        # Add custom mapping
        factory.add_mapping(
            "p_sq",
            "with_average_p1_probability",
            float,
            "Single-qubit error",
        )

        config = {"p_sq": 0.001}
        builder = factory.create_from_dict(config)
        assert isinstance(builder, GeneralNoiseModelBuilder)

    def test_custom_converter(self) -> None:
        """Test custom mapping with converter."""
        factory = GeneralNoiseFactory()

        # Add mapping with percentage converter
        def percent_to_prob(percent: float) -> float:
            return percent / 100.0

        factory.add_mapping(
            "p1_percent",
            "with_p1_probability",
            percent_to_prob,
            "P1 as percentage",
        )

        config = {"p1_percent": 0.1}  # 0.1% = 0.001
        builder = factory.create_from_dict(config)
        assert isinstance(builder, GeneralNoiseModelBuilder)

    def test_defaults(self) -> None:
        """Test setting and applying defaults."""
        factory = GeneralNoiseFactory()

        # Set defaults
        factory.set_default("p1", 0.001)
        factory.set_default("p2", 0.01)
        factory.set_default("seed", 42)

        # Empty config should use defaults
        builder = factory.create_from_dict({})
        assert isinstance(builder, GeneralNoiseModelBuilder)

        # User values should override defaults
        builder2 = factory.create_from_dict({"p1": 0.002, "seed": 123})
        assert isinstance(builder2, GeneralNoiseModelBuilder)

    def test_no_defaults(self) -> None:
        """Test disabling default application."""
        factory = GeneralNoiseFactory()
        factory.set_default("p1", 0.001)

        # With defaults disabled, empty config should still work
        builder = factory.create_from_dict({}, apply_defaults=False)
        assert isinstance(builder, GeneralNoiseModelBuilder)

    def test_validation_errors(self) -> None:
        """Test validation error reporting."""
        factory = GeneralNoiseFactory()

        config = {
            "p1": "not_a_number",  # Type error
            "unknown_key": 123,  # Unknown key
        }

        errors = factory.validate_config(config)
        assert "unknown_keys" in errors
        assert "p1" in errors

    def test_validation_success(self) -> None:
        """Test successful validation."""
        factory = GeneralNoiseFactory()

        config = {
            "seed": 42,
            "p1": 0.001,
            "p2": 0.01,
        }

        errors = factory.validate_config(config)
        assert errors == {}

    def test_get_available_keys(self) -> None:
        """Test retrieving available configuration keys."""
        factory = GeneralNoiseFactory()
        keys = factory.get_available_keys()

        # Check some expected keys
        assert "seed" in keys
        assert "p1" in keys
        assert "p2" in keys
        assert "p_meas_0" in keys
        assert "p_meas_1" in keys
        assert "noiseless_gates" in keys

        # Check descriptions
        assert "Random seed" in keys["seed"]
        assert "Single-qubit" in keys["p1"]

    def test_json_creation(self) -> None:
        """Test creating from JSON string."""
        factory = GeneralNoiseFactory()

        json_config = json.dumps(
            {
                "seed": 42,
                "p1": 0.001,
                "p2": 0.01,
                "scale": 1.2,
            },
        )

        builder = factory.create_from_json(json_config)
        assert isinstance(builder, GeneralNoiseModelBuilder)

    def test_complex_configuration(self) -> None:
        """Test complex configuration with many features."""
        factory = GeneralNoiseFactory()

        config = {
            "seed": 42,
            "scale": 1.5,
            "leakage_scale": 0.1,
            "p1_average": 0.001,
            "p1_pauli_model": {"X": 0.6, "Y": 0.2, "Z": 0.2},
            "p2_average": 0.008,
            "p2_pauli_model": {"IX": 0.25, "XI": 0.25, "XX": 0.5},
            "noiseless_gates": ["H", "S", "T"],
            "p_prep": 0.0005,
            "p_meas_0": 0.002,
            "p_meas_1": 0.003,
        }

        builder = factory.create_from_dict(config)
        assert isinstance(builder, GeneralNoiseModelBuilder)

    def test_use_defaults_parameter(self) -> None:
        """Test the use_defaults parameter."""
        # With defaults (default behavior)
        factory_with = GeneralNoiseFactory(use_defaults=True)
        assert len(factory_with.mappings) == 43  # Should have all standard mappings
        assert "p1" in factory_with.mappings
        assert "p2" in factory_with.mappings

        # Without defaults
        factory_without = GeneralNoiseFactory(use_defaults=False)
        assert len(factory_without.mappings) == 0  # Should be empty
        assert "p1" not in factory_without.mappings

    def test_class_method_constructors(self) -> None:
        """Test the with_defaults() and empty() class methods."""
        # Test with_defaults()
        factory_defaults = GeneralNoiseFactory.with_defaults()
        assert len(factory_defaults.mappings) == 43
        assert "p1" in factory_defaults.mappings

        # Test empty()
        factory_empty = GeneralNoiseFactory.empty()
        assert len(factory_empty.mappings) == 0
        assert "p1" not in factory_empty.mappings

    def test_override_warning(self) -> None:
        """Test that overriding default mappings produces a warning."""
        factory = GeneralNoiseFactory()

        # Capture warnings
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")

            # Override a default mapping
            factory.add_mapping("p1", "with_p2_probability", float)

            # Should have generated a warning
            assert len(w) == 1
            assert "Overriding default mapping" in str(w[0].message)
            assert "'p1'" in str(w[0].message)
            assert "with_p1_probability" in str(w[0].message)
            assert "with_p2_probability" in str(w[0].message)

    def test_no_warning_on_empty_factory(self) -> None:
        """Test that empty factory doesn't warn on 'overrides'."""
        factory = GeneralNoiseFactory.empty()

        # Capture warnings
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")

            # Add mapping (not an override since factory is empty)
            factory.add_mapping("p1", "with_p2_probability", float)

            # Should NOT generate a warning
            assert len(w) == 0

    def test_no_warning_on_new_key(self) -> None:
        """Test that adding new keys doesn't generate warnings."""
        factory = GeneralNoiseFactory()

        # Capture warnings
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")

            # Add new mapping (not an override)
            factory.add_mapping("custom_key", "with_p1_probability", float)

            # Should NOT generate a warning
            assert len(w) == 0

    def test_show_mappings_output(self, capsys: "pytest.CaptureFixture[str]") -> None:
        """Test the show_mappings method output."""
        factory = GeneralNoiseFactory()

        # Add an override to test the marker

        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            factory.add_mapping("p1", "with_p2_probability", float)

        # Set a default value
        factory.set_default("p1", 0.001)

        # Show mappings
        factory.show_mappings(show_descriptions=False)

        # Capture output
        captured = capsys.readouterr()

        # Check output contains expected elements
        assert "Current Parameter Mappings:" in captured.out
        assert "Configuration Key" in captured.out
        assert "Builder Method" in captured.out
        assert "*p1" in captured.out  # Should be marked as overridden
        assert "with_p2_probability" in captured.out
        assert "Default Values:" in captured.out
        assert "p1: 0.001" in captured.out
        assert "* = Overridden default mapping" in captured.out

    def test_empty_factory_usage(self) -> None:
        """Test using an empty factory with custom mappings."""
        factory = GeneralNoiseFactory.empty()

        # Add custom mappings
        factory.add_mapping("error_rate", "with_p1_probability", float)
        factory.add_mapping("two_qubit_error", "with_p2_probability", float)
        factory.add_mapping("random_seed", "with_seed", int)

        # Use custom config
        config = {
            "random_seed": 42,
            "error_rate": 0.001,
            "two_qubit_error": 0.01,
        }

        builder = factory.create_from_dict(config)
        assert isinstance(builder, GeneralNoiseModelBuilder)

    def test_strict_mode_with_empty_factory(self) -> None:
        """Test that strict mode works correctly with empty factory."""
        factory = GeneralNoiseFactory.empty()
        factory.add_mapping("my_key", "with_p1_probability", float)

        # Unknown key should raise in strict mode
        with pytest.raises(ValueError, match="Unknown configuration keys") as exc_info:
            factory.create_from_dict({"my_key": 0.001, "unknown": 0.002}, strict=True)

        assert "Unknown configuration keys" in str(exc_info.value)
        assert "unknown" in str(exc_info.value)

    def test_remove_mapping(self) -> None:
        """Test removing parameter mappings."""
        factory = GeneralNoiseFactory()

        # Remove an existing mapping
        assert "p1_average" in factory.mappings
        result = factory.remove_mapping("p1_average")
        assert result is True
        assert "p1_average" not in factory.mappings

        # Try to remove non-existent mapping
        result = factory.remove_mapping("does_not_exist")
        assert result is False

        # Verify removed key is no longer valid
        with pytest.raises(ValueError, match="Unknown configuration keys") as exc_info:
            factory.create_from_dict({"p1_average": 0.001}, strict=True)
        assert "Unknown configuration keys" in str(exc_info.value)
        assert "p1_average" in str(exc_info.value)

    def test_remove_mappings(self) -> None:
        """Test removing mappings from factory."""
        factory = GeneralNoiseFactory()

        # We can remove mappings if we don't want them
        assert "p1_average" in factory.mappings
        factory.remove_mapping("p1_average")
        assert "p1_average" not in factory.mappings

        # Try to use removed mapping
        with pytest.raises(ValueError, match="Unknown configuration keys"):
            factory.create_from_dict({"p1_average": 0.001}, strict=True)

        # But other mappings still work
        config = {
            "p_prep": 0.0005,
            "p_meas_0": 0.002,
            "p_meas_1": 0.003,
            "p1": 0.001,
            "p2": 0.01,
        }
        builder = factory.create_from_dict(config)
        assert isinstance(builder, GeneralNoiseModelBuilder)

    def test_custom_factory_scenario(self) -> None:
        """Test creating a custom factory with specific terminology."""
        # Start with empty factory
        factory = GeneralNoiseFactory.empty()

        # Add only the mappings we want with our terminology
        factory.add_mapping(
            "single_gate_error",
            "with_p1_probability",
            float,
            "Error rate for single-qubit gates",
        )
        factory.add_mapping(
            "two_gate_error",
            "with_p2_probability",
            float,
            "Error rate for two-qubit gates",
        )
        factory.add_mapping(
            "readout_error",
            "with_meas_0_probability",
            float,
            "Readout error (0->1)",
        )
        factory.add_mapping("seed", "with_seed", int, "Random seed")

        # Use our custom config
        config = {
            "seed": 42,
            "single_gate_error": 0.001,
            "two_gate_error": 0.01,
            "readout_error": 0.002,
        }

        builder = factory.create_from_dict(config)
        assert isinstance(builder, GeneralNoiseModelBuilder)

        # Standard keys should NOT work
        with pytest.raises(ValueError, match="Unknown configuration keys"):
            factory.create_from_dict({"p1": 0.001}, strict=True)


class TestConvenienceFunctions:
    """Test the convenience functions."""

    def test_create_noise_from_dict(self) -> None:
        """Test the convenience function for dict creation."""
        config = {
            "seed": 42,
            "p1": 0.001,
            "p2": 0.01,
        }

        builder = create_noise_from_dict(config)
        assert isinstance(builder, GeneralNoiseModelBuilder)

    def test_create_noise_from_json(self) -> None:
        """Test the convenience function for JSON creation."""
        json_config = '{"seed": 42, "p1": 0.001, "p2": 0.01}'

        builder = create_noise_from_json(json_config)
        assert isinstance(builder, GeneralNoiseModelBuilder)


class TestIonTrapNoiseFactory:
    """Test the specialized IonTrapNoiseFactory."""

    def test_ion_trap_defaults(self) -> None:
        """Test that ion trap factory has appropriate defaults."""
        factory = IonTrapNoiseFactory()

        # Should have ion trap specific defaults
        assert "p_prep" in factory.defaults
        assert "p1" in factory.defaults
        assert "p2" in factory.defaults
        assert "p_meas_0" in factory.defaults
        assert "p_meas_1" in factory.defaults

        # Check typical ion trap values
        assert (
            factory.defaults["p1"] < factory.defaults["p2"]
        )  # Single-qubit better than two-qubit
        assert (
            factory.defaults["p_meas_0"] < factory.defaults["p_meas_1"]
        )  # Dark state error < bright state

    def test_motional_heating_mapping(self) -> None:
        """Test the custom motional heating mapping."""
        factory = IonTrapNoiseFactory()

        config = {
            "seed": 42,
            "motional_heating": 2.0,  # Should be converted to scale
        }

        builder = factory.create_from_dict(config)
        assert isinstance(builder, GeneralNoiseModelBuilder)

    def test_ion_trap_inheritance(self) -> None:
        """Test that ion trap factory inherits all base functionality."""
        factory = IonTrapNoiseFactory()

        # Should have all standard mappings
        keys = factory.get_available_keys()
        assert "seed" in keys
        assert "p1" in keys
        assert "motional_heating" in keys


class TestAllBuilderMethods:
    """Test that all builder methods exposed through PyO3 work correctly."""

    def test_all_with_methods_callable(self) -> None:
        """Test that all with_* methods in the factory have corresponding callable builder methods."""
        from pecos_rslib import GeneralNoiseModelBuilder

        factory = GeneralNoiseFactory()
        builder = GeneralNoiseModelBuilder()

        # Get all builder methods
        builder_methods = {m for m in dir(builder) if m.startswith("with_")}

        # Check each factory mapping corresponds to a real method
        for key, mapping in factory.mappings.items():
            method_name = mapping.method_name
            assert (
                method_name in builder_methods
            ), f"Method {method_name} for key '{key}' not found in builder"

            # Verify the method is callable
            method = getattr(builder, method_name)
            assert callable(method), f"Method {method_name} is not callable"

    def test_each_with_method_works(self) -> None:
        """Test that each with_* method can be called successfully with appropriate values."""
        # Test data for each method type
        test_configs = {
            # Global parameters
            "seed": 42,
            "scale": 1.5,
            "leakage_scale": 0.5,
            "emission_scale": 0.3,
            "seepage_prob": 0.1,
            "noiseless_gate": "H",
            "noiseless_gates": ["H", "X", "CX"],
            # Idle noise
            "p_idle_coherent": True,
            "p_idle_linear_rate": 0.001,
            "p_idle_average_linear_rate": 0.0005,
            "p_idle_linear_model": {"X": 0.3, "Y": 0.3, "Z": 0.4},
            "p_idle_quadratic_rate": 0.0001,
            "p_idle_average_quadratic_rate": 0.00005,
            "p_idle_coherent_to_incoherent_factor": 2.0,
            "idle_scale": 0.8,
            # Preparation
            "p_prep": 0.001,
            "p_prep_leak_ratio": 0.1,
            "p_prep_crosstalk": 0.0001,
            "prep_scale": 0.9,
            "p_prep_crosstalk_scale": 0.5,
            # Single-qubit
            "p1": 0.001,
            "p1_average": 0.0008,
            "p1_emission_ratio": 0.05,
            "p1_emission_model": {"X": 0.5, "Y": 0.3, "Z": 0.2},
            "p1_seepage_prob": 0.02,
            "p1_pauli_model": {"X": 0.5, "Y": 0.3, "Z": 0.2},
            "p1_scale": 1.1,
            # Two-qubit
            "p2": 0.01,
            "p2_average": 0.008,
            "p2_angle_params": (0.8, 0.1, 1.2, 0.2),
            "p2_angle_power": 2.0,
            "p2_emission_ratio": 0.06,
            "p2_emission_model": {"IX": 0.25, "XI": 0.25, "XX": 0.5},
            "p2_seepage_prob": 0.03,
            "p2_pauli_model": {"IX": 0.25, "XI": 0.25, "XX": 0.5},
            "p2_idle": 0.0005,
            "p2_scale": 1.2,
            # Measurement
            "p_meas": 0.002,
            "p_meas_0": 0.002,
            "p_meas_1": 0.003,
            "p_meas_crosstalk": 0.0001,
            "meas_scale": 0.95,
            "p_meas_crosstalk_scale": 0.7,
        }

        factory = GeneralNoiseFactory()

        # Test each parameter individually
        for key, value in test_configs.items():
            try:
                factory.create_from_dict({key: value})
                # If we get here, the method call succeeded
                assert True, f"Successfully created builder with {key}={value}"
            except (ValueError, TypeError, AttributeError, KeyError) as e:
                pytest.fail(f"Failed to apply {key}={value}: {e!s}")

        # Test all parameters together
        try:
            factory.create_from_dict(test_configs)
            assert True, "Successfully created builder with all parameters"
        except (ValueError, TypeError, AttributeError, KeyError) as e:
            pytest.fail(f"Failed to apply all parameters together: {e!s}")

    def test_method_parameter_validation(self) -> None:
        """Test that builder methods validate their parameters correctly."""
        factory = GeneralNoiseFactory()

        # Test probability bounds validation
        # Rust panics raise BaseException
        with pytest.raises(BaseException, match="must be between 0 and 1"):
            factory.create_from_dict({"p1": -0.1})

        with pytest.raises(BaseException, match="must be between 0 and 1"):
            factory.create_from_dict({"p2": 1.5})

        with pytest.raises(BaseException, match="must be between 0 and 1"):
            factory.create_from_dict({"p_meas_0": 2.0})

        # Note: scale and idle_scale don't have validation in the current implementation
        # They accept any float value, including negative

        # Test positive validation
        with pytest.raises(BaseException, match="must be positive"):
            factory.create_from_dict({"p_idle_coherent_to_incoherent_factor": 0.0})

        with pytest.raises(BaseException, match="must be positive"):
            factory.create_from_dict({"p2_angle_power": -1.0})

        # Test unknown gate type
        with pytest.raises(ValueError, match="Invalid gate type"):
            factory.create_from_dict({"noiseless_gate": "INVALID_GATE"})


class TestIntegration:
    """Integration tests with actual simulation."""

    def test_factory_with_simulation(self) -> None:
        """Test using factory-created noise with actual simulation."""
        from pecos_rslib import qasm_engine, sim
        from pecos_rslib._pecos_rslib import QasmProgram

        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
        """

        # Create noise using factory
        factory = GeneralNoiseFactory()
        noise = factory.create_from_dict(
            {
                "seed": 42,
                "p1": 0.001,
                "p2": 0.01,
                "p_meas_0": 0.002,
                "p_meas_1": 0.002,
            },
        )

        # Create program and engine
        program = QasmProgram.from_string(qasm)
        engine = qasm_engine().program(program)

        # Run simulation
        results = sim(program).classical(engine).noise(noise).run(100).to_dict()

        # Should get results
        assert "c" in results
        assert len(results["c"]) == 100

        # With noise, should see some errors (not all 00 or 11)
        # Results are returned as a list of integers (bit representation)
        unique_results = set(results["c"])
        # With 2 qubits, possible values are 0 (00), 1 (01), 2 (10), 3 (11)
        # Perfect Bell state would only give 0 and 3, noise should introduce 1 and 2
        assert len(unique_results) >= 2  # Should see at least 2 different outcomes
