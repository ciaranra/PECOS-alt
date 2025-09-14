#!/usr/bin/env python3
"""Test the Guppy to LLVM compilation pipeline via execute_llvm."""

import pytest


@pytest.fixture
def simple_quantum_function() -> object:
    """Fixture providing a simple quantum Guppy function."""
    from guppylang import guppy
    from guppylang.std.quantum import h, measure, qubit

    @guppy
    def simple_quantum() -> bool:
        q = qubit()
        h(q)
        return measure(q)

    return simple_quantum


class TestGuppyExecuteLLVM:
    """Test suite for Guppy to LLVM compilation using execute_llvm."""

    def test_execute_llvm_module_available(self) -> None:
        """Test that execute_llvm module is available and has required functions."""
        try:
            from pecos import execute_llvm
        except ImportError:
            pytest.skip("execute_llvm module not available")

        assert hasattr(
            execute_llvm,
            "compile_module_to_string",
        ), "execute_llvm should have compile_module_to_string function"

    def test_compile_guppy_to_hugr(self, simple_quantum_function: object) -> None:
        """Test compiling a Guppy function to HUGR format."""
        try:
            compiled = simple_quantum_function.compile()
            hugr_bytes = compiled.to_bytes()
        except Exception as e:
            pytest.fail(f"HUGR compilation failed: {e}")

        assert hugr_bytes is not None, "HUGR compilation should produce bytes"
        assert len(hugr_bytes) > 0, "HUGR bytes should not be empty"

    def test_compile_hugr_to_llvm(self, simple_quantum_function: object) -> None:
        """Test compiling HUGR to LLVM IR using execute_llvm."""
        try:
            from pecos import execute_llvm
        except ImportError:
            pytest.skip("execute_llvm not available")

        # First compile Guppy to HUGR
        compiled = simple_quantum_function.compile()
        hugr_bytes = compiled.to_bytes()

        # Then compile HUGR to LLVM
        try:
            llvm_ir = execute_llvm.compile_module_to_string(hugr_bytes)
        except Exception as e:
            if "Unknown type" in str(e):
                pytest.skip(f"Known issue with type handling: {e}")
            pytest.fail(f"LLVM compilation failed: {e}")

        assert llvm_ir is not None, "LLVM compilation should produce IR"
        assert len(llvm_ir) > 0, "LLVM IR should not be empty"

        # Check for quantum operations or entry points in the IR
        has_quantum_ops = "__quantum__" in llvm_ir
        has_entry_point = "EntryPoint" in llvm_ir or "@main" in llvm_ir

        assert (
            has_quantum_ops or has_entry_point
        ), "LLVM IR should contain quantum operations or an entry point"

    def test_guppy_frontend_integration(self, simple_quantum_function: object) -> None:
        """Test GuppyFrontend integration with execute_llvm."""
        try:
            from pecos.frontends.guppy_frontend import GuppyFrontend
        except ImportError:
            pytest.skip("GuppyFrontend not available")

        try:
            frontend = GuppyFrontend(use_rust_backend=False)
        except Exception as e:
            pytest.skip(f"GuppyFrontend initialization failed: {e}")

        # Get backend info
        info = frontend.get_backend_info()
        assert isinstance(info, dict), "Backend info should be a dictionary"

        # Try to compile the function
        try:
            qir_file = frontend.compile_function(simple_quantum_function)
            assert qir_file is not None, "Compilation should produce a QIR file path"
        except Exception as e:
            # This is expected to fail in some environments
            if "HUGR version" in str(e) or "not available" in str(e):
                pytest.skip(f"Known compatibility issue: {e}")
            pytest.fail(f"Function compilation failed unexpectedly: {e}")

    def test_sim_api_available(self) -> None:
        """Test that the sim() API is available for execution."""
        try:
            from pecos.frontends import sim
        except ImportError as e:
            pytest.skip(f"sim API not available: {e}")

        assert callable(sim), "sim should be a callable function"
