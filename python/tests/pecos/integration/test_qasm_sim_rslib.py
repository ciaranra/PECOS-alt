"""Integration tests for qasm_sim using pecos.rslib imports."""

from collections import Counter


class TestQasmSimRslib:
    """Test qasm_sim functionality using pecos.rslib imports."""

    def test_import_qasm_sim(self) -> None:
        """Test that we can import qasm_sim from pecos.rslib."""
        from pecos.rslib import qasm_sim

        assert callable(qasm_sim)

    def test_import_noise_models(self) -> None:
        """Test that we can import noise models from pecos.rslib."""
        from pecos.rslib import (
            BiasedDepolarizingNoise,
            DepolarizingCustomNoise,
            DepolarizingNoise,
            GeneralNoise,
            PassThroughNoise,
        )

        # Test that we can instantiate them
        assert PassThroughNoise() is not None
        assert DepolarizingNoise(p=0.01) is not None
        assert (
            DepolarizingCustomNoise(p_prep=0.01, p_meas=0.01, p1=0.01, p2=0.02)
            is not None
        )
        assert BiasedDepolarizingNoise(p=0.01) is not None
        assert GeneralNoise() is not None

    def test_import_utilities(self) -> None:
        """Test that we can import utility functions from pecos.rslib."""
        from pecos.rslib import QuantumEngine, get_noise_models, get_quantum_engines

        noise_models = get_noise_models()
        assert isinstance(noise_models, list)
        assert "PassThrough" in noise_models
        assert "Depolarizing" in noise_models

        engines = get_quantum_engines()
        assert isinstance(engines, list)
        assert "StateVector" in engines
        assert "SparseStabilizer" in engines

        # Test QuantumEngine enum
        assert hasattr(QuantumEngine, "StateVector")
        assert hasattr(QuantumEngine, "SparseStabilizer")

    def test_basic_simulation(self) -> None:
        """Test basic QASM simulation using pecos.rslib imports."""
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

        results = qasm_sim(qasm).seed(42).run(1000)

        assert isinstance(results, dict)
        assert "c" in results
        assert len(results["c"]) == 1000

        # Check Bell state results
        counts = Counter(results["c"])
        assert set(counts.keys()) <= {0, 3}  # Only |00> and |11>
        assert all(count > 400 for count in counts.values())  # Roughly equal

    def test_simulation_with_noise(self) -> None:
        """Test QASM simulation with noise using pecos.rslib imports."""
        from pecos.rslib import DepolarizingNoise, qasm_sim

        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        x q[0];
        measure q[0] -> c[0];
        """

        # With noise
        results = qasm_sim(qasm).seed(42).noise(DepolarizingNoise(p=0.1)).run(1000)

        assert isinstance(results, dict)
        assert "c" in results
        assert len(results["c"]) == 1000

        # Should see some errors due to noise
        zeros = sum(1 for val in results["c"] if val == 0)
        assert 50 < zeros < 200  # Some bit flips due to noise

    def test_builder_pattern(self) -> None:
        """Test the builder pattern using pecos.rslib imports."""
        from pecos.rslib import BiasedDepolarizingNoise, QuantumEngine, qasm_sim

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

        # Build once
        sim = (
            qasm_sim(qasm)
            .seed(42)
            .workers(2)
            .noise(BiasedDepolarizingNoise(p=0.01))
            .quantum_engine(QuantumEngine.SparseStabilizer)
            .build()
        )

        # Run multiple times
        results1 = sim.run(100)
        results2 = sim.run(200)

        assert len(results1["c"]) == 100
        assert len(results2["c"]) == 200

        # Both should have the same types of results (GHZ state)
        counts1 = Counter(results1["c"])
        counts2 = Counter(results2["c"])

        # With low noise, should mostly see |000> and |111>
        assert 0 in counts1
        assert 7 in counts1
        assert 0 in counts2
        assert 7 in counts2

    def test_binary_string_format(self) -> None:
        """Test binary string format output using pecos.rslib imports."""
        from pecos.rslib import qasm_sim

        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[3];
        creg c[3];
        x q[0];
        x q[2];
        measure q -> c;
        """

        # Test binary string format
        results = qasm_sim(qasm).with_binary_string_format().run(10)

        assert isinstance(results, dict)
        assert "c" in results
        assert len(results["c"]) == 10

        # Check that all results are binary strings
        assert all(isinstance(val, str) for val in results["c"])
        assert all(len(val) == 3 for val in results["c"])
        assert all(set(val) <= {"0", "1"} for val in results["c"])

        # Should always measure |101>
        assert all(val == "101" for val in results["c"])

    def test_auto_workers(self) -> None:
        """Test auto_workers functionality using pecos.rslib imports."""
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

        # This should use all available CPU cores
        results = qasm_sim(qasm).auto_workers().run(1000)

        assert isinstance(results, dict)
        assert "c" in results
        assert len(results["c"]) == 1000

    def test_run_qasm_function(self) -> None:
        """Test the run_qasm function using pecos.rslib imports."""
        from pecos.rslib import DepolarizingNoise, QuantumEngine, run_qasm

        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
        """

        # Simple usage
        results = run_qasm(qasm, shots=100)
        assert len(results["c"]) == 100

        # With all parameters
        results = run_qasm(
            qasm,
            shots=100,
            noise_model=DepolarizingNoise(p=0.01),
            engine=QuantumEngine.StateVector,
            workers=2,
            seed=42,
        )
        assert len(results["c"]) == 100

    def test_large_register(self) -> None:
        """Test simulation with large quantum registers using pecos.rslib imports."""
        from pecos.rslib import qasm_sim

        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[100];
        creg c[100];
        x q[0];
        x q[50];
        x q[99];
        measure q -> c;
        """

        # Test with default format (should handle big integers)
        results = qasm_sim(qasm).run(5)

        assert "c" in results
        assert len(results["c"]) == 5

        # The result should have bits set at positions 0, 50, and 99
        # In integer form, this is 2^0 + 2^50 + 2^99
        expected = (1 << 0) + (1 << 50) + (1 << 99)
        assert all(val == expected for val in results["c"])

        # Test with binary string format
        results_binary = qasm_sim(qasm).with_binary_string_format().run(5)

        assert all(len(val) == 100 for val in results_binary["c"])
        # Check specific bit positions (remember: MSB first in string)
        for binary_str in results_binary["c"]:
            assert binary_str[99] == "1"  # q[0] -> position 99
            assert binary_str[49] == "1"  # q[50] -> position 49
            assert binary_str[0] == "1"  # q[99] -> position 0
            assert binary_str.count("1") == 3
