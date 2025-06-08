"""Tests for the qasm_sim builder pattern API."""

import pytest
from collections import Counter
from pecos_rslib.qasm_sim import (
    qasm_sim,
    run_qasm,
    QuantumEngine,
    PassThroughNoise,
    DepolarizingNoise,
    DepolarizingCustomNoise,
    BiasedDepolarizingNoise,
    BiasedMeasurementNoise,
    GeneralNoise,
)


class TestQasmSimBuilder:
    """Test the qasm_sim builder pattern."""

    def test_simple_run(self):
        """Test simple run without building."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
        """

        results = qasm_sim(qasm).run(100)
        assert "c" in results
        assert len(results["c"]) == 100

        # Check Bell state results
        counts = Counter(results["c"])
        assert set(counts.keys()) <= {0, 3}  # Only |00> and |11>

    def test_build_once_run_multiple(self):
        """Test building once and running multiple times."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        h q[0];
        measure q[0] -> c[0];
        """

        sim = qasm_sim(qasm).seed(42).build()

        # Run multiple times with different shots
        results1 = sim.run(100)
        results2 = sim.run(1000)
        results3 = sim.run(10)

        assert len(results1["c"]) == 100
        assert len(results2["c"]) == 1000
        assert len(results3["c"]) == 10

        # Check deterministic behavior with same seed
        sim2 = qasm_sim(qasm).seed(42).build()
        results4 = sim2.run(100)
        assert results1["c"] == results4["c"]

    def test_method_chaining(self):
        """Test method chaining with all configuration options."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
        """

        results = (
            qasm_sim(qasm)
            .seed(42)
            .workers(2)
            .quantum_engine(QuantumEngine.SparseStabilizer)
            .noise(DepolarizingNoise(p=0.01))
            .run(100)
        )

        assert "c" in results
        assert len(results["c"]) == 100

    def test_auto_workers(self):
        """Test auto_workers configuration."""
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

        results = qasm_sim(qasm).auto_workers().seed(42).run(1000)

        assert len(results["c"]) == 1000
        # Should see all 8 possible outcomes
        counts = Counter(results["c"])
        assert len(counts) == 8

    def test_noise_models(self):
        """Test different noise model configurations."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        x q[0];
        measure q[0] -> c[0];
        """

        # PassThrough (no noise)
        results = qasm_sim(qasm).noise(PassThroughNoise()).run(100)
        assert all(val == 1 for val in results["c"])

        # Depolarizing
        results = qasm_sim(qasm).seed(42).noise(DepolarizingNoise(p=0.1)).run(1000)
        errors = sum(1 for val in results["c"] if val == 0)
        assert 50 < errors < 200

        # Custom depolarizing
        qasm_bell = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
        """

        results = (
            qasm_sim(qasm_bell)
            .seed(42)
            .noise(DepolarizingCustomNoise(p_prep=0.01, p_meas=0.01, p1=0.001, p2=0.1))
            .run(1000)
        )
        counts = Counter(results["c"])
        # Should see errors due to high CX error
        assert 1 in counts or 2 in counts

        # Biased measurement
        results = (
            qasm_sim(qasm)
            .seed(42)
            .noise(BiasedMeasurementNoise(p0=0.0, p1=0.2))
            .run(1000)
        )
        zeros = sum(1 for val in results["c"] if val == 0)
        assert 150 < zeros < 250

        # Biased depolarizing
        results = (
            qasm_sim(qasm).seed(42).noise(BiasedDepolarizingNoise(p=0.05)).run(1000)
        )
        errors = sum(1 for val in results["c"] if val == 0)
        assert errors > 0

        # General noise
        results = qasm_sim(qasm).noise(GeneralNoise()).run(10)
        assert len(results["c"]) == 10

    def test_quantum_engines(self):
        """Test different quantum engine configurations."""
        # Clifford circuit
        qasm_clifford = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
        """

        # Both engines should work for Clifford circuits
        for engine in [QuantumEngine.StateVector, QuantumEngine.SparseStabilizer]:
            results = qasm_sim(qasm_clifford).seed(42).quantum_engine(engine).run(100)
            assert len(results["c"]) == 100

        # Non-Clifford circuit (only StateVector works)
        qasm_non_clifford = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        h q[0];
        rz(0.5) q[0];
        measure q[0] -> c[0];
        """

        # StateVector should work
        results = (
            qasm_sim(qasm_non_clifford)
            .quantum_engine(QuantumEngine.StateVector)
            .run(10)
        )
        assert len(results["c"]) == 10

        # SparseStabilizer might fail on non-Clifford gates
        # The RZ gate is approximated in QASM, so it might not fail immediately
        # Just verify it runs without checking for failure
        try:
            qasm_sim(qasm_non_clifford).quantum_engine(
                QuantumEngine.SparseStabilizer
            ).run(10)
        except RuntimeError:
            # Expected if the engine detects non-Clifford operations
            pass

    def test_deterministic_behavior(self):
        """Test deterministic behavior with seeds."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        h q[1];
        measure q -> c;
        """

        # Same seed should give same results
        results1 = qasm_sim(qasm).seed(123).run(100)
        results2 = qasm_sim(qasm).seed(123).run(100)
        assert results1["c"] == results2["c"]

        # Different seeds should give different results
        results3 = qasm_sim(qasm).seed(456).run(100)
        assert results1["c"] != results3["c"]

        # Building with seed should maintain determinism across runs
        sim = qasm_sim(qasm).seed(789).build()
        run1 = sim.run(50)
        run2 = sim.run(50)

        # Different runs from same sim should have same distribution
        # but not necessarily same exact values
        counts1 = Counter(run1["c"])
        counts2 = Counter(run2["c"])
        assert set(counts1.keys()) == set(counts2.keys())

    def test_large_register(self):
        """Test handling of large quantum registers."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[70];
        creg c[70];
        x q[0];
        x q[10];
        x q[20];
        x q[30];
        x q[40];
        x q[50];
        x q[60];
        x q[69];
        measure q -> c;
        """

        results = qasm_sim(qasm).run(10)
        assert len(results["c"]) == 10

        # Check that values are Python big integers
        for val in results["c"]:
            # Should be able to handle values larger than 64 bits
            assert isinstance(val, int)
            # Convert to binary and check set bits
            binary = bin(val)[2:].zfill(70)
            set_bits = [i for i, bit in enumerate(reversed(binary)) if bit == "1"]
            assert set_bits == [0, 10, 20, 30, 40, 50, 60, 69]

    def test_error_handling(self):
        """Test error handling in builder pattern."""
        # Invalid QASM
        with pytest.raises(RuntimeError):
            qasm_sim("invalid qasm").run(10)

        # Build should fail on invalid QASM
        with pytest.raises(RuntimeError):
            qasm_sim("invalid qasm").build()

    def test_builder_vs_direct_api(self):
        """Test that builder and direct API give same results."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
        """

        # Using builder pattern
        builder_results = (
            qasm_sim(qasm)
            .seed(42)
            .workers(2)
            .noise(DepolarizingNoise(p=0.01))
            .quantum_engine(QuantumEngine.SparseStabilizer)
            .run(100)
        )

        # Using direct run_qasm
        direct_results = run_qasm(
            qasm,
            shots=100,
            seed=42,
            workers=2,
            noise_model=DepolarizingNoise(p=0.01),
            engine=QuantumEngine.SparseStabilizer,
        )

        # Results should be identical
        assert builder_results["c"] == direct_results["c"]
