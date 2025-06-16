"""Test custom noise model registration and from_config pattern."""

from dataclasses import dataclass
from typing import Any

import pytest


class TestCustomNoiseModels:
    """Test custom noise model registration and configuration."""

    def test_built_in_noise_from_config(self) -> None:
        """Test that all built-in noise models have from_config methods."""
        from pecos.rslib import (
            BiasedDepolarizingNoise,
            DepolarizingCustomNoise,
            DepolarizingNoise,
            GeneralNoise,
            PassThroughNoise,
        )

        # Test PassThroughNoise
        pt = PassThroughNoise.from_config({})
        assert isinstance(pt, PassThroughNoise)

        # Test DepolarizingNoise with default
        dep1 = DepolarizingNoise.from_config({})
        assert dep1.p == 0.001  # default

        # Test DepolarizingNoise with custom value
        dep2 = DepolarizingNoise.from_config({"p": 0.05})
        assert dep2.p == 0.05

        # Test DepolarizingCustomNoise with mixed defaults and custom
        dep_custom = DepolarizingCustomNoise.from_config(
            {
                "p_prep": 0.002,
                "p1": 0.003,
                # p_meas and p2 should use defaults
            },
        )
        assert dep_custom.p_prep == 0.002
        assert dep_custom.p_meas == 0.001  # default
        assert dep_custom.p1 == 0.003
        assert dep_custom.p2 == 0.002  # default

        # Test BiasedDepolarizingNoise
        biased = BiasedDepolarizingNoise.from_config({"p": 0.1})
        assert biased.p == 0.1

        # Test GeneralNoise
        general = GeneralNoise.from_config({})
        assert isinstance(general, GeneralNoise)

    def test_register_custom_noise_model_limitation(self) -> None:
        """Test that custom noise models have limitations due to Rust bindings."""
        from pecos.rslib import qasm_sim, register_noise_model

        # Define a custom noise model
        @dataclass
        class MyCustomNoise:
            """Custom noise model for testing."""

            error_rate: float = 0.01
            gate_specific: bool = False

            @classmethod
            def from_config(cls, config: dict[str, Any]) -> "MyCustomNoise":
                """Create from configuration dictionary."""
                return cls(
                    error_rate=config.get("error_rate", 0.01),
                    gate_specific=config.get("gate_specific", False),
                )

        # Register it
        register_noise_model("MyCustomNoise", MyCustomNoise)

        # Use it in configuration
        qasm = """
            OPENQASM 2.0;
            include "qelib1.inc";
            qreg q[1];
            creg c[1];
            x q[0];
            measure q[0] -> c[0];
            """
        config = {
            "noise": {
                "type": "MyCustomNoise",
                "error_rate": 0.05,
                "gate_specific": True,
            },
        }

        # This will fail because custom Python noise models can't be passed to Rust
        with pytest.raises(ValueError, match="unknown variant `MyCustomNoise`"):
            qasm_sim(qasm).config(config).build()

    def test_register_without_from_config_fails(self) -> None:
        """Test that registering a class without from_config fails."""
        from pecos.rslib import register_noise_model

        # Define a class without from_config
        @dataclass
        class BadNoise:
            """Noise model without from_config method."""

            p: float = 0.01

        # Should raise ValueError
        with pytest.raises(ValueError, match="must have a 'from_config' classmethod"):
            register_noise_model("BadNoise", BadNoise)

    def test_override_existing_noise_model(self) -> None:
        """Test that we can override an existing noise model's configuration parsing."""
        from pecos.rslib import (
            DepolarizingNoise,
            qasm_sim,
            register_noise_model,
        )

        # The key insight: we can override how configs are parsed, but the result
        # must still be one of the built-in noise types that Rust understands

        # Define a custom configuration parser that returns a built-in type
        class CustomDepolarizingParser:
            """Custom parser that changes default values."""

            @classmethod
            def from_config(cls, config: dict[str, Any]) -> DepolarizingNoise:
                """Create DepolarizingNoise with different defaults."""
                # Use 0.1 as default instead of 0.001
                return DepolarizingNoise(p=config.get("p", 0.1))

        # Override the existing DepolarizingNoise parser
        register_noise_model("DepolarizingNoise", CustomDepolarizingParser)

        # Use it - this creates a real DepolarizingNoise with p=0.1
        qasm = """
            OPENQASM 2.0;
            include "qelib1.inc";
            qreg q[1];
            creg c[1];
            x q[0];
            measure q[0] -> c[0];
            """
        config = {
            "noise": {"type": "DepolarizingNoise"},  # Uses our custom parser
        }

        # This should work because it returns a real DepolarizingNoise
        sim = qasm_sim(qasm).config(config).build()
        results = sim.run(10)
        assert len(results["c"]) == 10

    def test_noise_config_validation(self) -> None:
        """Test that built-in noise models work with configuration."""
        from pecos.rslib import qasm_sim

        # Valid configuration should work with built-in noise models
        qasm_valid = """
            OPENQASM 2.0;
            include "qelib1.inc";
            qreg q[1];
            creg c[1];
            x q[0];
            measure q[0] -> c[0];
            """

        # Test DepolarizingNoise with valid p
        config_valid = {
            "noise": {"type": "DepolarizingNoise", "p": 0.5},
        }
        sim = qasm_sim(qasm_valid).config(config_valid).build()
        results = sim.run(10)
        assert len(results["c"]) == 10

        # Test DepolarizingCustomNoise with valid parameters
        config_custom = {
            "noise": {
                "type": "DepolarizingCustomNoise",
                "p_prep": 0.1,
                "p_meas": 0.2,
                "p1": 0.3,
                "p2": 0.4,
            },
        }
        sim = qasm_sim(qasm_valid).config(config_custom).build()
        results = sim.run(10)
        assert len(results["c"]) == 10

        # Test that unknown noise types fail
        config_invalid = {
            "noise": {"type": "UnknownNoiseType", "p": 0.5},
        }

        with pytest.raises(ValueError, match="unknown variant"):
            qasm_sim(qasm_valid).config(config_invalid).build()
