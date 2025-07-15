"""Comprehensive tests for qasm_sim covering all features and edge cases."""

from collections import Counter

import pytest


class TestQasmSimComprehensive:
    """Comprehensive tests for all qasm_sim features."""

    def test_pass_through_noise(self) -> None:
        """Test PassThroughNoise (no noise) produces deterministic results."""
        from pecos.rslib import PassThroughNoise, qasm_sim

        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        x q[0];
        x q[1];
        measure q -> c;
        """

        # With PassThroughNoise, results should be deterministic
        results = qasm_sim(qasm).noise(PassThroughNoise()).run(100)

        # Should always measure |11> = 3
        assert all(val == 3 for val in results["c"])

    def test_general_noise(self) -> None:
        """Test GeneralNoise model."""
        from pecos.rslib import GeneralNoise, qasm_sim

        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
        """

        # GeneralNoise uses default configuration
        results = qasm_sim(qasm).seed(42).noise(GeneralNoise()).run(1000)

        assert isinstance(results, dict)
        assert "c" in results
        assert len(results["c"]) == 1000

    def test_state_vector_engine(self) -> None:
        """Test StateVector engine explicitly."""
        from pecos.rslib import QuantumEngine, qasm_sim

        # Use a circuit with T gate (non-Clifford)
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        t q[0];
        cx q[0], q[1];
        measure q -> c;
        """

        results = (
            qasm_sim(qasm).quantum_engine(QuantumEngine.StateVector).seed(42).run(1000)
        )

        assert len(results["c"]) == 1000
        # Results should be probabilistic due to T gate
        counts = Counter(results["c"])
        assert len(counts) > 1  # Should see multiple outcomes

    def test_sparse_stabilizer_engine(self) -> None:
        """Test SparseStabilizer engine explicitly with Clifford circuit."""
        from pecos.rslib import QuantumEngine, qasm_sim

        # Pure Clifford circuit
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[3];
        creg c[3];
        h q[0];
        cx q[0], q[1];
        cx q[1], q[2];
        s q[2];
        measure q -> c;
        """

        results = (
            qasm_sim(qasm)
            .quantum_engine(QuantumEngine.SparseStabilizer)
            .seed(42)
            .run(1000)
        )

        assert len(results["c"]) == 1000

    def test_multiple_registers(self) -> None:
        """Test circuits with multiple classical registers."""
        from pecos.rslib import qasm_sim

        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[4];
        creg c1[2];
        creg c2[2];
        x q[0];
        x q[2];
        measure q[0] -> c1[0];
        measure q[1] -> c1[1];
        measure q[2] -> c2[0];
        measure q[3] -> c2[1];
        """

        results = qasm_sim(qasm).run(10)

        assert "c1" in results
        assert "c2" in results
        assert len(results["c1"]) == 10
        assert len(results["c2"]) == 10
        # c1 should always be |10> = 1
        assert all(val == 1 for val in results["c1"])
        # c2 should always be |10> = 1
        assert all(val == 1 for val in results["c2"])

    def test_empty_circuit(self) -> None:
        """Test empty circuit (no gates, just measurements)."""
        from pecos.rslib import qasm_sim

        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        measure q -> c;
        """

        results = qasm_sim(qasm).run(100)

        # Should always measure |00> = 0
        assert all(val == 0 for val in results["c"])

    def test_no_measurements(self) -> None:
        """Test circuit with no measurements."""
        from pecos.rslib import qasm_sim

        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        h q[0];
        cx q[0], q[1];
        """

        results = qasm_sim(qasm).run(100)

        # Should return empty dict when no measurements
        assert results == {}

    def test_partial_measurements(self) -> None:
        """Test measuring only some qubits."""
        from pecos.rslib import qasm_sim

        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[4];
        creg c[2];
        x q[0];
        x q[1];
        x q[2];
        x q[3];
        measure q[0] -> c[0];
        measure q[2] -> c[1];
        """

        results = qasm_sim(qasm).run(50)

        assert len(results["c"]) == 50
        # Should measure |11> = 3 (only q[0] and q[2])
        assert all(val == 3 for val in results["c"])

    def test_one_shot(self) -> None:
        """Test running with just 1 shot."""
        from pecos.rslib import qasm_sim

        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        x q[0];
        x q[1];
        measure q -> c;
        """

        results = qasm_sim(qasm).run(1)

        assert "c" in results
        assert len(results["c"]) == 1
        assert results["c"][0] == 3  # Should measure |11>

    def test_high_noise_probability(self) -> None:
        """Test with very high noise probability."""
        from pecos.rslib import DepolarizingNoise, qasm_sim

        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        x q[0];
        measure q[0] -> c[0];
        """

        # With 50% depolarizing noise
        results = qasm_sim(qasm).seed(42).noise(DepolarizingNoise(p=0.5)).run(1000)

        zeros = sum(1 for val in results["c"] if val == 0)
        # Should see significant errors, roughly 50/50 distribution
        assert 300 < zeros < 700

    def test_all_noise_models_in_config(self) -> None:
        """Test all noise models through qasm_sim config method."""
        from pecos.rslib import qasm_sim

        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        x q[0];
        measure q[0] -> c[0];
        """

        noise_configs = [
            {"type": "PassThroughNoise"},
            {"type": "GeneralNoise"},
            {"type": "DepolarizingNoise", "p": 0.1},
            {"type": "BiasedDepolarizingNoise", "p": 0.1},
            {
                "type": "DepolarizingCustomNoise",
                "p_prep": 0.1,
                "p_meas": 0.1,
                "p1": 0.1,
                "p2": 0.1,
            },
        ]

        for noise_config in noise_configs:
            config = {"seed": 42, "noise": noise_config}
            sim = qasm_sim(qasm).config(config).build()
            results = sim.run(100)
            assert len(results["c"]) == 100

    def test_binary_string_format_empty_register(self) -> None:
        """Test binary string format with empty measurements."""
        from pecos.rslib import qasm_sim

        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        h q[0];
        """

        results = qasm_sim(qasm).with_binary_string_format().run(10)
        assert results == {}  # No measurements

    def test_deterministic_with_seed(self) -> None:
        """Test that same seed produces same results."""
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

        # Use config dict that includes seed
        config1 = {
            "seed": 123,
            "noise": {"type": "DepolarizingNoise", "p": 0.01},
        }

        config2 = {
            "seed": 123,
            "noise": {"type": "DepolarizingNoise", "p": 0.01},
        }

        # Build and run simulations with same config
        sim1 = qasm_sim(qasm).config(config1).build()
        sim2 = qasm_sim(qasm).config(config2).build()

        results1 = sim1.run(1000)
        results2 = sim2.run(1000)

        # Should produce identical results with same seed
        assert results1["c"] == results2["c"]

        # Run with different seed
        config3 = {
            "seed": 456,
            "noise": {"type": "DepolarizingNoise", "p": 0.01},
        }
        sim3 = qasm_sim(qasm).config(config3).build()
        results3 = sim3.run(1000)

        # Should produce different results (with very high probability)
        # Count occurrences to verify they're different
        from collections import Counter

        counts1 = Counter(results1["c"])
        counts3 = Counter(results3["c"])

        # With 1000 shots and noise, the exact counts should differ
        assert counts1 != counts3

    def test_config_with_null_noise(self) -> None:
        """Test config with null noise field."""
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
            "noise": None,  # Explicitly null
        }

        sim = qasm_sim(qasm).config(config).build()
        results = sim.run(10)

        # Should work without noise
        assert all(val == 1 for val in results["c"])

    def test_invalid_qasm_syntax(self) -> None:
        """Test handling of invalid QASM syntax."""
        from pecos.rslib import qasm_sim

        invalid_qasm = """
        OPENQASM 2.0;
        invalid syntax here
        """

        with pytest.raises(RuntimeError):
            qasm_sim(invalid_qasm).run(10)
