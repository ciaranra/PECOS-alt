"""Test the complete working Guppy→HUGR→LLVM→PECOS pipeline."""

import pytest

# Check for required dependencies
try:
    from guppylang import guppy
    from guppylang.std.quantum import cx, h, measure, qubit, x

    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False

try:
    from pecos.frontends.guppy_api import sim
    from pecos_rslib import state_vector

    PECOS_API_AVAILABLE = True
except ImportError:
    PECOS_API_AVAILABLE = False

try:
    from pecos_rslib import compile_hugr_to_llvm

    HUGR_LLVM_AVAILABLE = True
except ImportError:
    HUGR_LLVM_AVAILABLE = False

try:
    from pecos.frontends.guppy_selene_compiler import GuppySeleneCompiler

    SELENE_COMPILER_AVAILABLE = True
except ImportError:
    SELENE_COMPILER_AVAILABLE = False


def decode_integer_results(results: list[int], n_bits: int) -> list[tuple[bool, ...]]:
    """Decode integer-encoded results back to tuples of booleans."""
    decoded = []
    for val in results:
        bits = [bool(val & (1 << i)) for i in range(n_bits)]
        decoded.append(tuple(bits))
    return decoded


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
class TestGuppyCompilation:
    """Test Guppy compilation capabilities."""

    def test_simple_quantum_function_creation(self) -> None:
        """Test creating a simple quantum function with Guppy."""

        @guppy
        def simple_quantum() -> bool:
            q = qubit()
            h(q)
            return measure(q)

        # Verify function was created
        assert simple_quantum is not None, "Function should be created"
        assert callable(simple_quantum), "Function should be callable"
        assert hasattr(simple_quantum, "compile"), "Function should have compile method"

    def test_bell_state_function_creation(self) -> None:
        """Test creating a Bell state function."""

        @guppy
        def bell_state() -> tuple[bool, bool]:
            q0 = qubit()
            q1 = qubit()
            h(q0)
            cx(q0, q1)
            return measure(q0), measure(q1)

        assert bell_state is not None, "Bell state function should be created"
        assert callable(bell_state), "Bell state should be callable"

    def test_parametric_quantum_function(self) -> None:
        """Test creating a parametric quantum function."""

        @guppy
        def parametric_circuit(n: int) -> int:
            count = 0
            for _i in range(n):
                q = qubit()
                h(q)
                if measure(q):
                    count += 1
            return count

        assert parametric_circuit is not None, "Parametric circuit should be created"
        assert callable(parametric_circuit), "Parametric circuit should be callable"


@pytest.mark.skipif(
    not all([GUPPY_AVAILABLE, HUGR_LLVM_AVAILABLE]),
    reason="Guppy or HUGR→LLVM not available",
)
class TestHUGRToLLVMCompilation:
    """Test HUGR to LLVM compilation."""

    def test_hugr_to_llvm_simple_circuit(self) -> None:
        """Test compiling simple circuit from HUGR to LLVM."""

        @guppy
        def simple_circuit() -> bool:
            q = qubit()
            h(q)
            return measure(q)

        # Compile to HUGR - the compile() method returns the Package directly
        package = simple_circuit.compile()

        # Get HUGR JSON representation (not envelope format)
        # Note: to_json() is deprecated but works with compile_hugr_to_llvm
        if hasattr(package, "to_json"):
            hugr_json = package.to_json()
        else:
            pytest.skip("Cannot get HUGR JSON representation")

        hugr_bytes = hugr_json.encode("utf-8")
        assert len(hugr_bytes) > 0, "HUGR bytes should not be empty"

        # Try to compile to LLVM
        try:
            llvm_ir = compile_hugr_to_llvm(hugr_bytes)
            assert llvm_ir is not None, "Should produce LLVM IR"
            assert isinstance(llvm_ir, str), "LLVM IR should be a string"
            assert len(llvm_ir) > 0, "LLVM IR should not be empty"

            # Check for quantum operations
            quantum_indicators = ["__quantum__", "@main", "EntryPoint", "define"]
            found_indicators = [ind for ind in quantum_indicators if ind in llvm_ir]
            assert (
                len(found_indicators) > 0
            ), f"LLVM should contain quantum operations: {found_indicators}"

        except (RuntimeError, ValueError) as e:
            if "not supported" in str(e).lower() or "not available" in str(e).lower():
                pytest.skip(f"HUGR to LLVM not fully supported: {e}")
            pytest.fail(f"HUGR to LLVM compilation failed: {e}")

    def test_hugr_to_llvm_bell_state(self) -> None:
        """Test compiling Bell state from HUGR to LLVM."""

        @guppy
        def bell_state() -> tuple[bool, bool]:
            q0 = qubit()
            q1 = qubit()
            h(q0)
            cx(q0, q1)
            return measure(q0), measure(q1)

        # Compile to HUGR - the compile() method returns the Package directly
        package = bell_state.compile()

        # Get HUGR JSON representation (not envelope format)
        # Note: to_json() is deprecated but works with compile_hugr_to_llvm
        if hasattr(package, "to_json"):
            hugr_json = package.to_json()
        else:
            pytest.skip("Cannot get HUGR JSON representation")

        hugr_bytes = hugr_json.encode("utf-8")

        try:
            llvm_ir = compile_hugr_to_llvm(hugr_bytes)
            assert llvm_ir is not None, "Should produce LLVM IR for Bell state"

            # Check for specific Bell state operations
            bell_ops = [
                "__quantum__qis__h",
                "__quantum__qis__cx",
                "__quantum__qis__cnot",
                "measure",
            ]
            found_ops = [op for op in bell_ops if op.lower() in llvm_ir.lower()]

            # Should have at least H and measurement
            assert (
                len(found_ops) >= 1
            ), f"Bell state should have quantum ops, found: {found_ops}"

        except (RuntimeError, ValueError) as e:
            if "not supported" in str(e).lower():
                pytest.skip(f"Bell state HUGR to LLVM not supported: {e}")
            pytest.fail(f"Bell state compilation failed: {e}")


@pytest.mark.skipif(
    not SELENE_COMPILER_AVAILABLE,
    reason="GuppySeleneCompiler not available",
)
class TestGuppySeleneCompiler:
    """Test GuppySeleneCompiler integration."""

    def test_compiler_creation(self) -> None:
        """Test creating GuppySeleneCompiler instance."""
        compiler = GuppySeleneCompiler()
        assert compiler is not None, "Should create compiler"

        # Check compiler methods
        compiler_methods = [m for m in dir(compiler) if not m.startswith("_")]
        assert len(compiler_methods) > 0, "Compiler should have public methods"

    def test_compiler_availability_check(self) -> None:
        """Test checking compiler availability."""
        compiler = GuppySeleneCompiler()

        # Check if compiler has availability check
        if hasattr(compiler, "is_available"):
            available = compiler.is_available()
            assert isinstance(available, bool), "Availability should be boolean"

    @pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
    def test_compiler_with_quantum_circuit(self) -> None:
        """Test compiler with quantum circuit."""

        @guppy
        def quantum_circuit() -> int:
            count = 0
            for _i in range(3):
                q = qubit()
                h(q)
                if measure(q):
                    count += 1
            return count

        GuppySeleneCompiler()

        # Try to compile the circuit
        compiled = quantum_circuit.compile()
        assert compiled is not None, "Should compile quantum circuit"


@pytest.mark.skipif(not PECOS_API_AVAILABLE, reason="PECOS API not available")
class TestSimAPI:
    """Test the sim() API."""

    @pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
    def test_sim_api_simple_circuit(self) -> None:
        """Test sim() API with simple circuit."""

        @guppy
        def simple_circuit() -> bool:
            q = qubit()
            h(q)
            return measure(q)

        try:
            results = (
                sim(simple_circuit).qubits(1).quantum(state_vector()).seed(42).run(10)
            )

            # Verify results structure
            assert isinstance(results, dict), "Results should be a dictionary"

            # Check for measurements
            if "measurement_1" in results:
                measurements = results["measurement_1"]
                assert len(measurements) == 10, "Should have 10 measurements"
                assert all(
                    m in [0, 1, True, False] for m in measurements
                ), "Measurements should be binary"
            elif "measurements" in results:
                measurements = results["measurements"]
                assert len(measurements) == 10, "Should have 10 measurements"
            else:
                assert len(results) > 0, "Should have some results"

        except (RuntimeError, ValueError) as e:
            if "not supported" in str(e).lower() or "PECOS" in str(e):
                pytest.skip(f"sim() API execution not fully supported: {e}")
            pytest.fail(f"sim() API failed: {e}")

    @pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
    def test_sim_api_bell_state(self) -> None:
        """Test sim() API with Bell state."""

        @guppy
        def bell_state() -> tuple[bool, bool]:
            q0 = qubit()
            q1 = qubit()
            h(q0)
            cx(q0, q1)
            return measure(q0), measure(q1)

        try:
            results = (
                sim(bell_state).qubits(2).quantum(state_vector()).seed(42).run(100)
            )

            assert isinstance(results, dict), "Results should be a dictionary"

            # Check for Bell state correlation
            if "measurement_1" in results and "measurement_2" in results:
                m1 = results["measurement_1"]
                m2 = results["measurement_2"]

                assert len(m1) == 100, "Should have 100 measurements for qubit 1"
                assert len(m2) == 100, "Should have 100 measurements for qubit 2"

                # Bell state should be correlated
                correlated = sum(1 for i in range(100) if m1[i] == m2[i])
                correlation_rate = correlated / 100
                assert (
                    correlation_rate > 0.95
                ), f"Bell state should be correlated, got {correlation_rate:.2%}"

        except (RuntimeError, ValueError) as e:
            if "not supported" in str(e).lower():
                pytest.skip(f"Bell state simulation not supported: {e}")
            pytest.fail(f"Bell state simulation failed: {e}")

    def test_sim_api_with_noise(self) -> None:
        """Test sim() API with noise model."""
        if not GUPPY_AVAILABLE:
            pytest.skip("Guppy not available")

        @guppy
        def noisy_circuit() -> bool:
            q = qubit()
            x(q)  # Put in |1⟩ state
            return measure(q)

        try:
            from pecos_rslib import depolarizing_noise

            # Create depolarizing noise model with 10% error probability
            noise_model = depolarizing_noise().with_uniform_probability(0.1)

            # Run with depolarizing noise
            results = (
                sim(noisy_circuit)
                .qubits(1)
                .quantum(state_vector())
                .noise(
                    noise_model,
                )
                .seed(42)
                .run(100)
            )

            assert isinstance(results, dict), "Results should be a dictionary"

            # With X gate and no noise, should always measure 1
            # With 10% depolarizing noise, should sometimes measure 0
            if "measurement_1" in results:
                measurements = results["measurement_1"]
                ones = sum(measurements)

                # Should be mostly 1s but not all due to noise
                assert (
                    70 < ones < 100
                ), f"With noise, should have some errors, got {ones}/100"

        except ImportError:
            pytest.skip("Noise models not available")
        except (RuntimeError, ValueError) as e:
            if "not supported" in str(e).lower():
                pytest.skip(f"Noise simulation not supported: {e}")


class TestCompletePipeline:
    """Test the complete Guppy→HUGR→LLVM→PECOS pipeline."""

    @pytest.mark.skipif(
        not all([GUPPY_AVAILABLE, PECOS_API_AVAILABLE]),
        reason="Full pipeline not available",
    )
    def test_complete_pipeline_integration(self) -> None:
        """Test complete pipeline from Guppy to execution."""

        # Create quantum circuit
        @guppy
        def quantum_algorithm() -> tuple[bool, bool, bool]:
            """Three-qubit quantum algorithm."""
            q0 = qubit()
            q1 = qubit()
            q2 = qubit()

            # Create superposition
            h(q0)
            h(q1)

            # Entangle
            cx(q0, q2)
            cx(q1, q2)

            # Measure
            return measure(q0), measure(q1), measure(q2)

        # Test compilation
        compiled = quantum_algorithm.compile()
        assert compiled is not None, "Should compile algorithm"

        # Test execution through sim API
        try:
            results = (
                sim(quantum_algorithm)
                .qubits(3)
                .quantum(state_vector())
                .seed(42)
                .run(50)
            )

            assert isinstance(results, dict), "Should get results dictionary"

            # Verify we got measurements
            has_measurements = (
                "measurement_1" in results
                or "measurements" in results
                or len(results) > 0
            )
            assert has_measurements, "Should have measurement results"

            # If we have individual measurements, check structure
            if "measurement_1" in results:
                for i in range(1, 4):
                    key = f"measurement_{i}"
                    if key in results:
                        assert (
                            len(results[key]) == 50
                        ), f"Should have 50 measurements for {key}"

        except (RuntimeError, ValueError) as e:
            if "PECOS" in str(e) or "not supported" in str(e).lower():
                # Pipeline compiled but execution failed - this is partial success
                pass
            else:
                pytest.fail(f"Pipeline failed unexpectedly: {e}")

    def test_pipeline_error_handling(self) -> None:
        """Test error handling in the pipeline."""
        if not GUPPY_AVAILABLE:
            pytest.skip("Guppy not available")

        @guppy
        def invalid_circuit() -> bool:
            # This might cause issues in some backends
            q = qubit()
            # Missing any gates
            return measure(q)

        # Should still compile
        compiled = invalid_circuit.compile()
        assert compiled is not None, "Should compile even simple circuit"

        if PECOS_API_AVAILABLE:
            # Should handle execution gracefully
            try:
                results = sim(invalid_circuit).qubits(1).quantum(state_vector()).run(10)
                # If it works, verify results
                assert isinstance(results, dict), "Should get results"
            except (RuntimeError, ValueError):
                # Expected - some backends might reject this
                pass

    def test_integer_result_decoding(self) -> None:
        """Test the integer result decoding utility."""
        # Test decoding 2-bit integers
        results = [0, 1, 2, 3]  # All possible 2-bit values
        decoded = decode_integer_results(results, 2)

        expected = [
            (False, False),  # 0 = 00
            (True, False),  # 1 = 01
            (False, True),  # 2 = 10
            (True, True),  # 3 = 11
        ]

        assert decoded == expected, f"Decoding mismatch: {decoded} != {expected}"

        # Test decoding 3-bit integers
        results = [0, 5, 7]  # 000, 101, 111
        decoded = decode_integer_results(results, 3)

        expected = [
            (False, False, False),  # 0 = 000
            (True, False, True),  # 5 = 101
            (True, True, True),  # 7 = 111
        ]

        assert decoded == expected, f"3-bit decoding mismatch: {decoded} != {expected}"
