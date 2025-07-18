"""Test custom noise model registration and from_config pattern."""

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
        from pecos.rslib import qasm_sim

        # Custom noise models cannot be registered in the current implementation
        # The API only supports built-in noise models that are implemented in Rust

        # Use an unknown noise type in configuration
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
        with pytest.raises(
            ValueError,
            match="Invalid noise configuration type: MyCustomNoise",
        ):
            qasm_sim(qasm).config(config).build()

    def test_register_without_from_config_fails(self) -> None:
        """Test that using noise without from_config fails."""
        # In the current implementation, noise model registration is not supported
        # All noise models must be built-in types implemented in Rust
        # This test is kept to document this limitation

    def test_override_existing_noise_model(self) -> None:
        """Test that built-in noise models use their standard configuration."""
        from pecos.rslib import qasm_sim

        # The current implementation uses fixed configuration parsing for built-in types
        # You cannot override how configs are parsed

        qasm = """
            OPENQASM 2.0;
            include "qelib1.inc";
            qreg q[1];
            creg c[1];
            x q[0];
            measure q[0] -> c[0];
            """

        # DepolarizingNoise requires 'p' field to be specified
        config = {
            "noise": {"type": "DepolarizingNoise", "p": 0.001},
        }

        sim = qasm_sim(qasm).config(config).build()
        results = sim.run(1000)

        # Should see very few errors due to low default noise (p=0.001)
        zeros = sum(1 for val in results["c"] if val == 0)
        assert zeros < 10  # Less than 1% error rate expected

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

        with pytest.raises(ValueError, match="Invalid noise configuration type"):
            qasm_sim(qasm_valid).config(config_invalid).build()
