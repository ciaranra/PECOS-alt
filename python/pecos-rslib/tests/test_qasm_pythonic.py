"""Tests for the new Pythonic QASM simulation interface."""

import pytest
from pecos_rslib.qasm_sim import (
    run_qasm,
    QuantumEngine,
    PassThroughNoise,
    DepolarizingNoise,
    DepolarizingCustomNoise,
    BiasedDepolarizingNoise,
    BiasedMeasurementNoise,
    GeneralNoise,
)


class TestPythonicInterface:
    """Test the new Pythonic QASM simulation interface."""

    def test_simple_run_qasm(self):
        """Test basic run_qasm functionality."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        x q[0];
        x q[1];
        measure q -> c;
        """

        # Run with minimal parameters
        results = run_qasm(qasm, shots=10)
        assert "c" in results
        assert len(results["c"]) == 10

        # All shots should measure 11 (both qubits in |1>)
        assert all(val == 3 for val in results["c"])  # 0b11 = 3

    def test_run_qasm_with_engine(self):
        """Test run_qasm with different engines."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        h q[0];
        measure q[0] -> c[0];
        """

        # Test with StateVector engine
        results_sv = run_qasm(
            qasm, shots=100, engine=QuantumEngine.StateVector, seed=42
        )
        assert "c" in results_sv
        assert len(results_sv["c"]) == 100

        # Test with SparseStabilizer engine
        results_stab = run_qasm(
            qasm, shots=100, engine=QuantumEngine.SparseStabilizer, seed=42
        )
        assert "c" in results_stab
        assert len(results_stab["c"]) == 100

    def test_run_qasm_with_noise_dataclasses(self):
        """Test run_qasm with noise model dataclasses."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        x q[0];
        measure q[0] -> c[0];
        """

        # Test with PassThroughNoise (no noise)
        results = run_qasm(qasm, shots=100, noise_model=PassThroughNoise())
        assert all(val == 1 for val in results["c"])

        # Test with DepolarizingNoise
        results = run_qasm(
            qasm, shots=1000, noise_model=DepolarizingNoise(p=0.3), seed=42
        )
        # With strong noise, should see some errors
        zeros = sum(1 for val in results["c"] if val == 0)
        assert 100 < zeros < 500  # Should see some bit flips

        # Test with BiasedMeasurementNoise
        results = run_qasm(
            qasm,
            shots=1000,
            noise_model=BiasedMeasurementNoise(p0=0.0, p1=0.2),
            seed=42,
        )
        # With p0=0 and p1=0.2, we should see ~20% of |1> states measured as 0
        zeros = sum(1 for val in results["c"] if val == 0)
        assert 100 < zeros < 300

    def test_run_qasm_with_custom_depolarizing(self):
        """Test run_qasm with custom depolarizing noise."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
        """

        # Custom depolarizing with different error rates
        noise = DepolarizingCustomNoise(
            p_prep=0.01,
            p_meas=0.02,
            p1=0.001,  # Low single-qubit error
            p2=0.1,  # High two-qubit error
        )

        results = run_qasm(qasm, shots=1000, noise_model=noise, seed=42)
        assert "c" in results
        assert len(results["c"]) == 1000

        # With CX error, should see some non-Bell states (01 and 10)
        from collections import Counter

        counts = Counter(results["c"])

        # Should see some errors due to high CX error rate
        assert 1 in counts or 2 in counts  # 01 or 10 states

    def test_run_qasm_deterministic(self):
        """Test deterministic behavior with seed."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        h q[0];
        measure q[0] -> c[0];
        """

        # Run twice with same seed
        results1 = run_qasm(qasm, shots=100, seed=123)
        results2 = run_qasm(qasm, shots=100, seed=123)

        # Results should be identical
        assert results1["c"] == results2["c"]

        # Different seed should give different results
        results3 = run_qasm(qasm, shots=100, seed=456)
        assert results1["c"] != results3["c"]  # With high probability

    def test_run_qasm_with_workers(self):
        """Test run_qasm with multiple workers."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[3];
        creg c[3];
        h q[0];
        h q[1];
        h q[2];
        measure q -> c;
        """

        # Run with multiple workers
        results = run_qasm(qasm, shots=1000, workers=4, seed=42)
        assert "c" in results
        assert len(results["c"]) == 1000

        # Check that we get a reasonable distribution
        from collections import Counter

        counts = Counter(results["c"])

        # Should see all 8 possible outcomes
        assert len(counts) == 8
        # Each outcome should appear roughly 125 times (1000/8)
        for count in counts.values():
            assert 50 < count < 200

    def test_noise_dataclass_defaults(self):
        """Test that noise dataclasses have sensible defaults."""
        # Check default values
        assert DepolarizingNoise().p == 0.001
        assert DepolarizingCustomNoise().p_prep == 0.001
        assert DepolarizingCustomNoise().p_meas == 0.001
        assert DepolarizingCustomNoise().p1 == 0.001
        assert DepolarizingCustomNoise().p2 == 0.002
        assert BiasedDepolarizingNoise().p == 0.001
        assert BiasedMeasurementNoise().p0 == 0.01
        assert BiasedMeasurementNoise().p1 == 0.01

    def test_error_handling(self):
        """Test error handling for invalid inputs."""
        # Invalid QASM should raise error
        with pytest.raises(RuntimeError):
            run_qasm("invalid qasm", shots=10)

        # Test with GeneralNoise (should work)
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        measure q[0] -> c[0];
        """
        results = run_qasm(qasm, shots=10, noise_model=GeneralNoise())
        assert "c" in results
        assert len(results["c"]) == 10
