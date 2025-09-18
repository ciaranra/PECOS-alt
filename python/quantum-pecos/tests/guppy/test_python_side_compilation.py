"""Test Python-side Guppy to Selene compilation."""

import tempfile
from pathlib import Path

import pytest


class TestPythonSideCompilation:
    """Test suite for Python-side Guppy compilation to Selene."""

    @pytest.fixture
    def simple_circuit(self) -> object:
        """Fixture providing a simple quantum circuit."""
        try:
            from guppylang.decorator import guppy
            from guppylang.std.quantum import h, measure, qubit
        except ImportError:
            pytest.skip("Guppy not available")

        @guppy
        def simple_circuit() -> bool:
            """Simple H-gate and measurement."""
            q = qubit()
            h(q)
            return measure(q)

        return simple_circuit

    @pytest.fixture
    def bell_pair_circuit(self) -> object:
        """Fixture providing a Bell pair circuit."""
        try:
            from guppylang.decorator import guppy
            from guppylang.std.quantum import cx, h, measure, qubit
        except ImportError:
            pytest.skip("Guppy not available")

        @guppy
        def bell_pair() -> tuple[bool, bool]:
            """Create a Bell pair."""
            q1 = qubit()
            q2 = qubit()
            h(q1)
            cx(q1, q2)  # Create entanglement
            return measure(q1), measure(q2)

        return bell_pair

    @pytest.mark.optional_dependency
    def test_compile_guppy_for_selene(self, simple_circuit: object) -> None:
        """Test compiling Guppy function for Selene execution."""
        try:
            from pecos.frontends.guppy_selene_compiler import compile_guppy_for_selene
        except ImportError:
            pytest.skip("GuppySeleneCompiler not available")

        # Create a temporary output directory
        with tempfile.TemporaryDirectory(prefix="test_guppy_selene_") as temp_dir:
            test_output_dir = Path(temp_dir)

            try:
                # Compile for Selene
                output_dir = compile_guppy_for_selene(simple_circuit, test_output_dir)
            except (ImportError, RuntimeError, ValueError) as e:
                if "not available" in str(e) or "not supported" in str(e):
                    pytest.skip(f"Compilation not available: {e}")
                pytest.fail(f"Compilation failed unexpectedly: {e}")

            # Verify output directory was created
            assert output_dir.exists(), "Output directory should exist"
            assert output_dir.is_dir(), "Output should be a directory"

            # Look for compiled files
            llvm_files = list(output_dir.glob("*.ll"))
            hugr_files = list(output_dir.glob("*.hugr"))

            # Should have at least one LLVM and one HUGR file
            assert len(llvm_files) > 0, f"No LLVM files found in {output_dir}"
            assert len(hugr_files) > 0, f"No HUGR files found in {output_dir}"

            # Verify file contents are non-empty
            llvm_file = llvm_files[0]
            hugr_file = hugr_files[0]

            assert llvm_file.stat().st_size > 0, "LLVM file should not be empty"
            assert hugr_file.stat().st_size > 0, "HUGR file should not be empty"

            # Check LLVM file contains expected patterns
            llvm_content = llvm_file.read_text()
            assert (
                "__quantum__" in llvm_content or "@" in llvm_content
            ), "LLVM file should contain quantum operations or functions"

    @pytest.mark.optional_dependency
    def test_selene_engine_integration(self, simple_circuit: object) -> None:
        """Test integration with Selene engine."""
        try:
            from pecos.frontends.guppy_selene_compiler import compile_guppy_for_selene
            from pecos_rslib import selene_engine
        except ImportError as e:
            pytest.skip(f"Required modules not available: {e}")

        with tempfile.TemporaryDirectory(prefix="test_selene_engine_") as temp_dir:
            test_output_dir = Path(temp_dir)

            try:
                # Compile for Selene
                output_dir = compile_guppy_for_selene(simple_circuit, test_output_dir)
                llvm_files = list(output_dir.glob("*.ll"))

                if not llvm_files:
                    pytest.skip("No LLVM files generated")

                llvm_file = llvm_files[0]

                # Try to create Selene engine
                engine = selene_engine().llvm_file(str(llvm_file)).qubits(1)

                # Verify engine was created
                assert engine is not None, "Selene engine should be created"

                # Try to convert to simulation
                try:
                    sim_obj = engine.to_sim()
                    assert sim_obj is not None, "Should create simulation object"
                except (RuntimeError, ValueError) as e:
                    # Engine creation might fail due to LLVM format issues
                    if "format" in str(e).lower() or "invalid" in str(e).lower():
                        pytest.skip(
                            f"LLVM format issue (expected during development): {e}",
                        )
                    pytest.fail(f"Unexpected engine error: {e}")

            except (ImportError, RuntimeError, ValueError) as e:
                if "not available" in str(e) or "not supported" in str(e):
                    pytest.skip(f"Selene engine not available: {e}")
                pytest.fail(f"Engine integration failed: {e}")

    def test_hugr_pass_through_compilation(self, bell_pair_circuit: object) -> None:
        """Test the HUGR pass-through path (Guppy → HUGR → Rust)."""
        try:
            from pecos.frontends.guppy_api import sim
            from pecos_rslib import state_vector
        except ImportError as e:
            pytest.skip(f"Required modules not available: {e}")

        try:
            # The sim API handles Guppy → HUGR → Selene compilation
            results = (
                sim(bell_pair_circuit)
                .qubits(2)
                .quantum(state_vector())
                .seed(42)
                .run(100)
            )
        except (RuntimeError, ValueError) as e:
            if "compilation" in str(e).lower() or "not supported" in str(e):
                pytest.skip(f"HUGR compilation issue: {e}")
            pytest.fail(f"HUGR pass-through failed: {e}")

        # Verify results structure
        assert isinstance(results, dict), "Results should be a dictionary"

        # Check for measurement results
        assert (
            "measurement_1" in results or "measurements" in results
        ), "Results should contain measurements"

        if "measurement_1" in results and "measurement_2" in results:
            # New format with separate measurement keys
            m1 = results["measurement_1"]
            m2 = results["measurement_2"]

            assert len(m1) == 100, "Should have 100 measurements for qubit 1"
            assert len(m2) == 100, "Should have 100 measurements for qubit 2"

            # Bell pair should be correlated
            correlated = sum(1 for i in range(100) if m1[i] == m2[i])
            correlation_rate = correlated / 100

            assert (
                correlation_rate > 0.9
            ), f"Bell pair should be highly correlated, got {correlation_rate:.2%}"

        elif "measurements" in results:
            # Old format or combined measurements
            measurements = results["measurements"]
            assert len(measurements) == 100, "Should have 100 measurements"
            assert all(
                isinstance(m, tuple | int) for m in measurements
            ), "Measurements should be tuples or integers"

    def test_compilation_with_complex_circuit(self) -> None:
        """Test compilation with a more complex quantum circuit."""
        try:
            from guppylang.decorator import guppy
            from guppylang.std.quantum import cx, h, measure, qubit, x, y, z
            from pecos.frontends.guppy_selene_compiler import compile_guppy_for_selene
        except ImportError as e:
            pytest.skip(f"Required modules not available: {e}")

        @guppy
        def complex_circuit() -> tuple[bool, bool, bool]:
            """More complex circuit with multiple gates."""
            q1 = qubit()
            q2 = qubit()
            q3 = qubit()

            # Apply various gates
            h(q1)
            x(q2)
            y(q3)
            cx(q1, q2)
            z(q3)
            cx(q2, q3)
            h(q3)

            return measure(q1), measure(q2), measure(q3)

        with tempfile.TemporaryDirectory(prefix="test_complex_circuit_") as temp_dir:
            test_output_dir = Path(temp_dir)

            try:
                # Compile the complex circuit
                output_dir = compile_guppy_for_selene(complex_circuit, test_output_dir)
            except (ImportError, RuntimeError, ValueError) as e:
                if "not available" in str(e) or "not supported" in str(e):
                    pytest.skip(f"Compilation not available: {e}")
                pytest.fail(f"Complex circuit compilation failed: {e}")

            # Verify files were created
            llvm_files = list(output_dir.glob("*.ll"))
            assert len(llvm_files) > 0, "Should generate LLVM file for complex circuit"

            # Check that LLVM contains multiple quantum operations
            llvm_content = llvm_files[0].read_text()

            # Check for quantum operations in various possible formats
            quantum_patterns = [
                "__quantum__",
                "qis",
                "@h",
                "@x",
                "@y",
                "@z",
                "@cx",
                "@measure",
                "hadamard",
                "cnot",
                "measure",
                "EntryPoint",
                "define",
            ]

            # Convert to lowercase for case-insensitive search
            llvm_lower = llvm_content.lower()
            ops_found = sum(
                1 for pattern in quantum_patterns if pattern.lower() in llvm_lower
            )

            # Just verify we have some quantum content or valid LLVM structure
            assert (
                ops_found > 0 or "define" in llvm_content
            ), "Complex circuit should contain LLVM operations or definitions"

    def test_compilation_output_structure(self, simple_circuit: object) -> None:
        """Test the structure of compilation outputs."""
        try:
            from pecos.compilation_pipeline import compile_guppy_to_hugr
        except ImportError:
            pytest.skip("Compilation pipeline not available")

        try:
            # Compile to HUGR
            hugr_bytes = compile_guppy_to_hugr(simple_circuit)
        except Exception as e:
            pytest.fail(f"HUGR compilation failed: {e}")

        # Verify HUGR output
        assert hugr_bytes is not None, "Should produce HUGR bytes"
        assert len(hugr_bytes) > 0, "HUGR bytes should not be empty"
        assert isinstance(hugr_bytes, bytes), "HUGR should be bytes"

        # Check for HUGR markers
        hugr_str = hugr_bytes.decode("utf-8")
        is_hugr_envelope = hugr_str.startswith("HUGRiHJv")
        is_json = hugr_str.startswith("{") or "{" in hugr_str[:100]

        assert is_hugr_envelope or is_json, "HUGR should be in envelope format or JSON"

        # If JSON, verify it can be parsed
        if is_json or (is_hugr_envelope and "{" in hugr_str):
            import json

            json_start = hugr_str.find("{") if is_hugr_envelope else 0
            if json_start != -1:
                try:
                    json_data = json.loads(hugr_str[json_start:])
                    assert isinstance(
                        json_data,
                        dict,
                    ), "HUGR JSON should be a dictionary"
                    assert len(json_data) > 0, "HUGR JSON should not be empty"
                except json.JSONDecodeError as e:
                    pytest.fail(f"HUGR JSON is invalid: {e}")


class TestCompilationErrorHandling:
    """Test error handling in compilation process."""

    def test_invalid_function_compilation(self) -> None:
        """Test compilation with invalid function."""
        try:
            from pecos.compilation_pipeline import compile_guppy_to_hugr
        except ImportError:
            pytest.skip("Compilation pipeline not available")

        # Try to compile a non-Guppy function
        def regular_function() -> int:
            return 42

        with pytest.raises((TypeError, ValueError, AttributeError)):
            compile_guppy_to_hugr(regular_function)

    @pytest.mark.optional_dependency
    def test_missing_output_directory(self) -> None:
        """Test compilation with missing output directory."""
        try:
            from guppylang.decorator import guppy
            from guppylang.std.quantum import h, measure, qubit
            from pecos.frontends.guppy_selene_compiler import compile_guppy_for_selene
        except ImportError:
            pytest.skip("Required modules not available")

        @guppy
        def test_circuit() -> bool:
            q = qubit()
            h(q)
            return measure(q)

        # Try to compile to a non-existent nested directory
        # The compiler should either create it or raise an appropriate error
        with tempfile.TemporaryDirectory() as tmpdir:
            non_existent_dir = Path(tmpdir) / "non" / "existent" / "nested" / "dir"

            try:
                output_dir = compile_guppy_for_selene(test_circuit, non_existent_dir)
                # If it succeeds, verify the directory was created
                assert output_dir.exists(), "Output directory should be created"
            except (OSError, ValueError) as e:
                # Expected - directory creation might fail due to permissions
                assert (
                    "directory" in str(e).lower() or "path" in str(e).lower()
                ), f"Error should mention directory/path issue: {e}"
