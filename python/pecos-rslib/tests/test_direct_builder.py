"""Test direct GeneralNoiseModelBuilder usage."""

import pytest
from collections import Counter
from pecos_rslib.qasm_sim import (
    qasm_sim,
    QuantumEngine,
    GeneralNoiseModelBuilder,
)


class TestDirectBuilder:
    """Test using GeneralNoiseModelBuilder directly."""

    def test_direct_builder_noise(self):
        """Test setting noise with GeneralNoiseModelBuilder directly using .noise() method."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
        """

        # Create and configure the Rust-native builder with fluent chaining
        builder = (
            GeneralNoiseModelBuilder()
            .with_seed(42)
            .with_p1_probability(0.001)
            .with_p2_probability(0.01)
            .with_meas_0_probability(0.002)
            .with_meas_1_probability(0.002)
        )

        # Use builder directly with .noise() method
        sim = qasm_sim(qasm).noise(builder).build()
        results = sim.run(1000)

        assert len(results["c"]) == 1000
        counts = Counter(results["c"])
        # Should see Bell state results (0 and 3) with some noise errors
        assert 0 in counts
        assert 3 in counts

    def test_builder_with_pauli_model(self):
        """Test builder with Pauli error models."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        x q[0];
        measure q[0] -> c[0];
        """

        builder = (
            GeneralNoiseModelBuilder()
            .with_seed(42)
            .with_p1_probability(0.1)  # High error rate for testing
            .with_p1_pauli_model({"X": 0.5, "Y": 0.3, "Z": 0.2})
        )

        results = qasm_sim(qasm).noise(builder).run(1000)

        # Should see some errors due to high p1 error rate
        zeros = sum(1 for val in results["c"] if val == 0)
        assert zeros > 50  # Some errors expected

    def test_builder_with_method_chaining(self):
        """Test using builder with direct method chaining."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
        """

        # Create builder with fluent API
        builder = GeneralNoiseModelBuilder().with_seed(42).with_p2_probability(0.01)

        # Use with direct method chaining
        sim = (
            qasm_sim(qasm)
            .seed(42)
            .workers(2)
            .noise(builder)
            .quantum_engine(QuantumEngine.StateVector)
            .with_binary_string_format()
            .build()
        )
        results = sim.run(100)

        assert len(results["c"]) == 100
        # Check binary string format
        assert all(isinstance(val, str) for val in results["c"])
        assert all(len(val) == 2 for val in results["c"])

    def test_builder_chaining_validation(self):
        """Test that builder methods validate parameters."""
        # Test validation
        with pytest.raises(ValueError, match="p1 must be between 0 and 1"):
            GeneralNoiseModelBuilder().with_p1_probability(1.5)

        with pytest.raises(ValueError, match="scale must be non-negative"):
            GeneralNoiseModelBuilder().with_scale(-1)

        with pytest.raises(ValueError, match="leakage_scale must be between 0 and 1"):
            GeneralNoiseModelBuilder().with_leakage_scale(1.5)

    def test_rust_vs_native_noise_models(self):
        """Test using Rust noise models in the .noise() method directly."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
        """

        # Create builder
        builder = GeneralNoiseModelBuilder()
        builder.with_seed(42)
        builder.with_p1_probability(0.001)
        builder.with_p2_probability(0.01)

        # Test that builder can be used directly in .noise() method
        sim = qasm_sim(qasm).noise(builder).seed(42).build()
        results = sim.run(100)

        assert len(results["c"]) == 100
        counts = Counter(results["c"])
        assert 0 in counts or 3 in counts  # Bell state results
