"""Test and document default values for qasm_sim."""


class TestQasmSimDefaults:
    """Test and document default values for all qasm_sim settings."""

    def test_builder_defaults(self) -> None:
        """Test and document defaults when using qasm_sim builder."""
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

        # Build with all defaults
        sim = qasm_sim(qasm).build()

        # Based on Rust code, the defaults are:
        # - seed: None (non-deterministic)
        # - workers: 1 (single thread)
        # - noise_model: PassThroughNoise (no noise)
        # - quantum_engine: SparseStabilizer
        # - bit_format: BigInt (integers, not binary strings)

        # Run to verify it works
        results = sim.run(100)
        assert len(results["c"]) == 100

    def test_run_qasm_defaults(self) -> None:
        """Test and document defaults when using run_qasm function."""
        from pecos.rslib import run_qasm

        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        x q[0];
        measure q[0] -> c[0];
        """

        # Run with minimal parameters
        results = run_qasm(qasm, shots=10)

        # Defaults for run_qasm:
        # - noise_model: None (no noise)
        # - engine: None (auto-selected based on circuit)
        # - workers: None (defaults to 1)
        # - seed: None (non-deterministic)

        assert all(val == 1 for val in results["c"])

    def test_noise_model_defaults(self) -> None:
        """Test and document default parameters for noise models."""
        from pecos.rslib import (
            BiasedDepolarizingNoise,
            DepolarizingCustomNoise,
            DepolarizingNoise,
        )

        # Test default values for noise models
        dep = DepolarizingNoise()
        assert dep.p == 0.001  # Default probability

        dep_custom = DepolarizingCustomNoise()
        assert dep_custom.p_prep == 0.001
        assert dep_custom.p_meas == 0.001
        assert dep_custom.p1 == 0.001
        assert dep_custom.p2 == 0.002  # Higher for 2-qubit gates

        biased = BiasedDepolarizingNoise()
        assert biased.p == 0.001

    def test_config_defaults(self) -> None:
        """Test and document defaults when using qasm_sim config method."""
        from pecos.rslib import qasm_sim

        # Minimal config - only required field
        qasm = """
            OPENQASM 2.0;
            include "qelib1.inc";
            qreg q[1];
            creg c[1];
            x q[0];
            measure q[0] -> c[0];
            """
        config = {}

        sim = qasm_sim(qasm).config(config).build()
        results = sim.run(10)

        # Defaults for qasm_sim with config method:
        # - seed: None (not set)
        # - workers: 1 (from builder default)
        # - noise: PassThroughNoise (no noise - ideal simulation)
        # - quantum_engine: SparseStabilizer (from builder default)
        # - binary_string_format: False (integers)

        assert all(val == 1 for val in results["c"])

    def test_no_noise_means_pass_through(self) -> None:
        """Test that omitting noise config results in PassThroughNoise (deterministic)."""
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

        # Config without noise specification
        config1 = {}

        # Config with explicit PassThroughNoise
        config2 = {
            "noise": {"type": "PassThroughNoise"},
        }

        # Both should produce identical deterministic results
        sim1 = qasm_sim(qasm).config(config1).build()
        sim2 = qasm_sim(qasm).config(config2).build()

        results1 = sim1.run(100)
        results2 = sim2.run(100)

        # Both should always measure |11> = 3
        assert all(val == 3 for val in results1["c"])
        assert all(val == 3 for val in results2["c"])

    def test_default_summary(self) -> None:
        """Document all defaults in one place."""
        # Default values summary:
        #
        # QasmSimulationBuilder defaults:
        # - seed: None (non-deterministic)
        # - workers: 1 (single thread)
        # - noise_model: PassThroughNoise (no noise)
        # - quantum_engine: SparseStabilizer
        # - bit_format: BigInt (integers, not binary strings)
        #
        # run_qasm function defaults:
        # - noise_model: None (no noise)
        # - engine: None (auto-selected)
        # - workers: None → 1 (single thread)
        # - seed: None (non-deterministic)
        #
        # Noise model parameter defaults:
        # - DepolarizingNoise.p: 0.001
        # - DepolarizingCustomNoise.p_prep: 0.001
        # - DepolarizingCustomNoise.p_meas: 0.001
        # - DepolarizingCustomNoise.p1: 0.001
        # - DepolarizingCustomNoise.p2: 0.002
        # - BiasedDepolarizingNoise.p: 0.001
        #
        # qasm_sim config method defaults:
        # - All optional fields use builder defaults when not specified
        # - noise: PassThroughNoise (no noise) when omitted

        # This test just documents the defaults
        assert True
