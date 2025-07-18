"""Tests for QASM simulation PyO3 bindings."""

import pytest
from pecos_rslib import (
    NoiseModel,
    QuantumEngine,
    run_qasm,
    get_noise_models,
    get_quantum_engines,
)


class TestQASMSimBindings:
    """Test the QASM simulation PyO3 bindings."""

    def test_noise_model_enum(self):
        """Test NoiseModel enum creation."""
        # Test valid noise models
        assert str(NoiseModel("PassThrough")) == "PassThrough"
        assert str(NoiseModel("Depolarizing")) == "Depolarizing"
        assert str(NoiseModel("DepolarizingCustom")) == "DepolarizingCustom"
        assert str(NoiseModel("BiasedDepolarizing")) == "BiasedDepolarizing"
        assert str(NoiseModel("General")) == "General"

        # Test case insensitive
        assert str(NoiseModel("passthrough")) == "PassThrough"
        assert str(NoiseModel("DEPOLARIZING")) == "Depolarizing"

        # Test invalid model
        with pytest.raises(ValueError, match="Unknown noise model type: invalid"):
            NoiseModel("invalid")

    def test_quantum_engine_enum(self):
        """Test QuantumEngine enum creation."""
        # Test valid engines
        assert str(QuantumEngine("StateVector")) == "StateVector"
        assert str(QuantumEngine("SparseStabilizer")) == "SparseStabilizer"

        # Test aliases
        assert str(QuantumEngine("state_vector")) == "StateVector"
        assert str(QuantumEngine("sv")) == "StateVector"
        assert str(QuantumEngine("stab")) == "SparseStabilizer"

        # Test invalid engine
        with pytest.raises(ValueError, match="Unknown quantum engine type: invalid"):
            QuantumEngine("invalid")

    def test_deterministic_simulation(self):
        """Test deterministic QASM simulation using seed parameter."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        h q[0];
        measure q[0] -> c[0];
        """

        # Run with same seed should give same results
        results1 = run_qasm(qasm, shots=100, seed=42)
        results2 = run_qasm(qasm, shots=100, seed=42)

        measurements1 = results1["c"]
        measurements2 = results2["c"]
        assert measurements1 == measurements2

        # Different seed should give different results (with high probability)
        results3 = run_qasm(qasm, shots=100, seed=123)
        measurements3 = results3["c"]
        # This might fail with very low probability
        assert measurements1 != measurements3

    def test_get_available_models(self):
        """Test getting available noise models and engines."""
        noise_models = get_noise_models()
        assert "PassThrough" in noise_models
        assert "Depolarizing" in noise_models
        assert len(noise_models) >= 5

        engines = get_quantum_engines()
        assert "StateVector" in engines
        assert "SparseStabilizer" in engines
        assert len(engines) == 2

    def test_error_handling(self):
        """Test error handling for invalid inputs."""
        # Invalid QASM
        with pytest.raises(RuntimeError):
            run_qasm("invalid qasm", shots=10)
