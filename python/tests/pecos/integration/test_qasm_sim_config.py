"""Test qasm_sim structured configuration functionality."""

import json
from collections import Counter

import pytest


class TestQasmSimStructuredConfig:
    """Test qasm_sim structured configuration functionality."""

    def test_basic_config(self) -> None:
        """Test basic configuration without noise."""
        from pecos.rslib import qasm_sim

        qasm = """
            OPENQASM 2.0;
            include "qelib1.inc";
            qreg q[2];
            creg c[2];
            h q[0];
            cx q[0], q[1];
            measure q -> c;
            """
        config = {"seed": 42}

        sim = qasm_sim(qasm).config(config).build()
        results = sim.run(1000)

        assert isinstance(results, dict)
        assert "c" in results
        assert len(results["c"]) == 1000

        # Check Bell state results
        counts = Counter(results["c"])
        assert set(counts.keys()) <= {0, 3}  # Only |00> and |11>

    def test_config_with_noise(self) -> None:
        """Test configuration with noise model."""
        from pecos.rslib import qasm_sim

        qasm = """
            OPENQASM 2.0;
            include "qelib1.inc";
            qreg q[1];
            creg c[1];
            x q[0];
            measure q[0] -> c[0];
            """
        config = {
            "seed": 42,
            "noise": {"type": "DepolarizingNoise", "p": 0.1},
        }

        sim = qasm_sim(qasm).config(config).build()
        results = sim.run(1000)

        # Should see some errors due to noise
        zeros = sum(1 for val in results["c"] if val == 0)
        assert 50 < zeros < 200  # Some bit flips due to noise

    def test_full_config(self) -> None:
        """Test configuration with all options."""
        from pecos.rslib import qasm_sim

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
        config = {
            "seed": 42,
            "workers": 2,
            "noise": {"type": "BiasedDepolarizingNoise", "p": 0.01},
            "quantum_engine": "SparseStabilizer",
            "binary_string_format": True,
        }

        sim = qasm_sim(qasm).config(config).build()
        results = sim.run(100)

        assert isinstance(results, dict)
        assert "c" in results
        assert len(results["c"]) == 100

        # Check binary string format
        assert all(isinstance(val, str) for val in results["c"])
        assert all(len(val) == 3 for val in results["c"])
        assert all(set(val) <= {"0", "1"} for val in results["c"])

    def test_auto_workers(self) -> None:
        """Test configuration with auto workers."""
        from pecos.rslib import qasm_sim

        qasm = """
            OPENQASM 2.0;
            include "qelib1.inc";
            qreg q[2];
            creg c[2];
            h q[0];
            cx q[0], q[1];
            measure q -> c;
            """
        config = {
            "workers": "auto",
        }

        sim = qasm_sim(qasm).config(config).build()
        results = sim.run(100)

        assert len(results["c"]) == 100

    def test_custom_noise_config(self) -> None:
        """Test configuration with custom noise parameters."""
        from pecos.rslib import qasm_sim

        qasm = """
            OPENQASM 2.0;
            include "qelib1.inc";
            qreg q[2];
            creg c[2];
            h q[0];
            cx q[0], q[1];
            measure q -> c;
            """
        config = {
            "seed": 42,
            "noise": {
                "type": "DepolarizingCustomNoise",
                "p_prep": 0.001,
                "p_meas": 0.002,
                "p1": 0.003,
                "p2": 0.004,
            },
        }

        sim = qasm_sim(qasm).config(config).build()
        results = sim.run(100)

        assert len(results["c"]) == 100

    def test_missing_qasm_raises_error(self) -> None:
        """Test that missing QASM code raises error."""
        # This test is no longer relevant since QASM is now a required parameter
        # to qasm_sim(), not part of the config

    def test_invalid_noise_type_raises_error(self) -> None:
        """Test that invalid noise type raises error."""
        from pecos.rslib import qasm_sim

        qasm = "OPENQASM 2.0;"
        config = {
            "noise": {"type": "InvalidNoise"},
        }

        with pytest.raises(ValueError, match="Invalid noise configuration"):
            qasm_sim(qasm).config(config).build()

    def test_invalid_engine_raises_error(self) -> None:
        """Test that invalid quantum engine raises error."""
        from pecos.rslib import qasm_sim

        qasm = "OPENQASM 2.0;"
        config = {
            "quantum_engine": "InvalidEngine",
        }

        with pytest.raises(ValueError, match="Unknown quantum engine"):
            qasm_sim(qasm).config(config).build()

    def test_json_serializable_config(self) -> None:
        """Test that configuration can be JSON serialized and deserialized."""
        from pecos.rslib import qasm_sim

        qasm = """
            OPENQASM 2.0;
            include "qelib1.inc";
            qreg q[2];
            creg c[2];
            h q[0];
            cx q[0], q[1];
            measure q -> c;
            """
        config = {
            "seed": 42,
            "workers": 4,
            "noise": {"type": "DepolarizingNoise", "p": 0.01},
            "quantum_engine": "SparseStabilizer",
            "binary_string_format": False,
        }

        # Serialize to JSON and back
        json_str = json.dumps(config)
        loaded_config = json.loads(json_str)

        # Should work the same way
        sim = qasm_sim(qasm).config(loaded_config).build()
        results = sim.run(100)

        assert len(results["c"]) == 100

    def test_structured_config(self) -> None:
        """Test new structured configuration approach."""
        from pecos.rslib import GeneralNoiseModelBuilder, QuantumEngine, qasm_sim

        qasm = """
            OPENQASM 2.0;
            include "qelib1.inc";
            qreg q[2];
            creg c[2];
            h q[0];
            cx q[0], q[1];
            measure q -> c;
            """

        # Create noise using builder - pass it directly to noise() method
        noise_builder = (
            GeneralNoiseModelBuilder()
            .with_seed(42)
            .with_p1_probability(0.001)
            .with_p2_probability(0.01)
        )

        # Use builder pattern instead of config dict
        sim = (
            qasm_sim(qasm)
            .seed(42)
            .auto_workers()
            .noise(noise_builder)
            .quantum_engine(QuantumEngine.StateVector)
            .with_binary_string_format()
            .build()
        )
        results = sim.run(100)

        assert isinstance(results, dict)
        assert "c" in results
        assert len(results["c"]) == 100

        # Check binary string format
        assert all(isinstance(val, str) for val in results["c"])
        assert all(len(val) == 2 for val in results["c"])

    def test_general_noise_config(self) -> None:
        """Test GeneralNoise configuration with dictionary."""
        from pecos.rslib import qasm_sim

        qasm = """
            OPENQASM 2.0;
            include "qelib1.inc";
            qreg q[2];
            creg c[2];
            h q[0];
            cx q[0], q[1];
            measure q -> c;
            """
        config = {
            "seed": 42,
            "noise": {
                "type": "GeneralNoise",
                "p1": 0.001,
                "p2": 0.01,
                "p_prep": 0.001,
                "p_meas_0": 0.002,
                "p_meas_1": 0.002,
                "noiseless_gates": ["H"],
                "p1_pauli_model": {
                    "X": 0.5,
                    "Y": 0.3,
                    "Z": 0.2,
                },
            },
        }

        sim = qasm_sim(qasm).config(config).build()
        results = sim.run(100)

        assert len(results["c"]) == 100
