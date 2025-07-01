"""Test structured configuration for qasm_sim with direct method chaining."""

import pytest
from collections import Counter
from pecos_rslib.qasm_sim import (
    qasm_sim,
    QuantumEngine,
    GeneralNoiseModelBuilder,
    DepolarizingNoise,
    DepolarizingCustomNoise,
    BiasedDepolarizingNoise,
    GeneralNoise,
)


class TestDirectMethodChaining:
    """Test the direct method chaining configuration approach."""

    def test_general_noise_model_builder_basic(self):
        """Test basic GeneralNoiseModelBuilder usage."""
        noise = (
            GeneralNoiseModelBuilder()
            .with_seed(42)
            .with_p1_probability(0.001)
            .with_p2_probability(0.01)
            .with_meas_0_probability(0.002)
            .with_meas_1_probability(0.002)
        )

        # Should be able to use the noise object
        assert hasattr(noise, "_get_builder")
        assert noise._get_builder() is not None

    def test_general_noise_model_builder_validation(self):
        """Test GeneralNoiseModelBuilder parameter validation."""
        builder = GeneralNoiseModelBuilder()

        # Test invalid probability values
        with pytest.raises(ValueError, match="p1 must be between 0 and 1"):
            builder.with_p1_probability(1.5)

        with pytest.raises(ValueError, match="scale must be non-negative"):
            builder.with_scale(-1.0)

        with pytest.raises(ValueError, match="leakage_scale must be between 0 and 1"):
            builder.with_leakage_scale(2.0)

    def test_general_noise_model_builder_advanced(self):
        """Test advanced GeneralNoiseModelBuilder features."""
        noise = (
            GeneralNoiseModelBuilder()
            .with_seed(42)
            .with_scale(1.5)
            .with_noiseless_gate("H")
            .with_p1_probability(0.001)
            .with_p1_pauli_model({"X": 0.5, "Y": 0.3, "Z": 0.2})
            .with_p2_probability(0.01)
            .with_prep_probability(0.0005)
            .with_meas_0_probability(0.002)
            .with_meas_1_probability(0.003)
        )

        # Should be able to use the noise object
        builder = noise._get_builder()
        assert builder is not None

    def test_general_noise_model_builder_with_simulation(self):
        """Test GeneralNoiseModelBuilder integration with qasm_sim."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
        """

        noise = (
            GeneralNoiseModelBuilder()
            .with_seed(42)
            .with_p1_probability(0.001)
            .with_p2_probability(0.01)
        )

        results = qasm_sim(qasm).seed(42).noise(noise).run(100)
        assert len(results["c"]) == 100

    def test_direct_method_chaining_basic(self):
        """Test basic direct method chaining configuration."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
        """

        # Test method chaining with various configurations
        results = (
            qasm_sim(qasm)
            .seed(42)
            .workers(4)
            .noise(DepolarizingNoise(p=0.01))
            .quantum_engine(QuantumEngine.StateVector)
            .with_binary_string_format()
            .run(100)
        )

        assert len(results["c"]) == 100
        # Check binary string format
        assert all(isinstance(val, str) for val in results["c"])
        assert all(len(val) == 2 for val in results["c"])

    def test_auto_workers_method(self):
        """Test auto_workers method."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        h q[0];
        measure q[0] -> c[0];
        """

        results = (
            qasm_sim(qasm)
            .seed(42)
            .auto_workers()  # Should automatically set workers based on CPU cores
            .run(100)
        )

        assert len(results["c"]) == 100

    def test_method_chaining_with_general_noise_builder(self):
        """Test method chaining with GeneralNoiseModelBuilder."""
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

        noise = (
            GeneralNoiseModelBuilder()
            .with_seed(42)
            .with_p1_probability(0.001)
            .with_p2_probability(0.008)
            .with_meas_0_probability(0.002)
            .with_meas_1_probability(0.002)
        )

        # Use chaining with custom noise
        sim = (
            qasm_sim(qasm)
            .seed(42)
            .workers(2)
            .noise(noise)
            .quantum_engine(QuantumEngine.StateVector)
            .build()
        )

        results = sim.run(100)
        assert len(results["c"]) == 100

    def test_general_noise_direct_usage(self):
        """Test using GeneralNoise dataclass directly."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
        """

        # Create noise directly
        noise = GeneralNoise(p1=0.001, p2=0.01, p_meas_0=0.002, p_meas_1=0.002)

        results = qasm_sim(qasm).seed(42).noise(noise).run(100)

        assert len(results["c"]) == 100

    def test_noise_model_comparison(self):
        """Test different noise models with method chaining."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        x q[0];
        measure q[0] -> c[0];
        """

        noise_models = [
            ("No noise", None),
            ("Depolarizing", DepolarizingNoise(p=0.1)),
            (
                "Custom depolarizing",
                DepolarizingCustomNoise(p_prep=0.01, p_meas=0.05, p1=0.02, p2=0.03),
            ),
            ("Biased depolarizing", BiasedDepolarizingNoise(p=0.1)),
        ]

        for name, noise in noise_models:
            if noise is None:
                results = qasm_sim(qasm).seed(42).run(1000)
            else:
                results = qasm_sim(qasm).seed(42).noise(noise).run(1000)

            # Count measurement errors (should see mostly 1s for X gate)
            zeros = sum(1 for val in results["c"] if val == 0)

            if name == "No noise":
                assert zeros == 0  # Perfect X gate
            else:
                assert zeros > 0  # Some errors expected

    def test_complex_noise_configuration(self):
        """Test complex noise configuration with method chaining."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[4];
        creg c[4];
        h q[0];
        h q[1];
        cx q[0], q[2];
        cx q[1], q[3];
        measure q -> c;
        """

        noise = (
            GeneralNoiseModelBuilder()
            .with_seed(42)
            .with_scale(1.2)
            .with_noiseless_gate("H")
            .with_p1_probability(0.0005)
            .with_p1_pauli_model({"X": 0.4, "Y": 0.3, "Z": 0.3})
            .with_p2_probability(0.005)
            .with_prep_probability(0.001)
            .with_meas_0_probability(0.002)
            .with_meas_1_probability(0.002)
        )

        results = (
            qasm_sim(qasm)
            .seed(42)
            .auto_workers()
            .noise(noise)
            .quantum_engine(QuantumEngine.StateVector)
            .with_binary_string_format()
            .run(1000)
        )

        assert len(results["c"]) == 1000
        # Check binary string format
        assert all(isinstance(val, str) for val in results["c"])
        assert all(len(val) == 4 for val in results["c"])

        # Verify we have some variety in results (not all same state)
        counts = Counter(results["c"])
        assert len(counts) > 1  # Should have multiple different measurement outcomes
