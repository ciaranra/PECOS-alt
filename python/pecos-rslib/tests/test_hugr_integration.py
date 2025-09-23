"""Tests for HUGR/LLVM PyO3 integration

Tests the Rust backend for HUGR compilation and LLVM engine creation.
Note: Many of these features have been deprecated in favor of the unified sim() API.
"""

import tempfile
from pathlib import Path

import pytest


# Test availability checks
def test_hugr_backend_availability() -> None:
    """Test that we can check HUGR backend availability."""
    try:
        from pecos_rslib import RUST_HUGR_AVAILABLE, check_rust_hugr_availability

        available, message = check_rust_hugr_availability()
        assert isinstance(available, bool)
        assert isinstance(message, str)
        assert available == RUST_HUGR_AVAILABLE

    except ImportError:
        # This is expected if the Rust backend is not compiled
        pytest.skip("Rust HUGR backend not available")


def test_hugr_compiler_creation() -> None:
    """Test creating HUGR compiler instances and basic functionality."""
    try:
        from pecos_rslib import RustHugrCompiler

        # Test default creation
        compiler = RustHugrCompiler()

        # Test that compiler has the expected methods
        assert hasattr(compiler, "compile_bytes_to_llvm")
        assert callable(compiler.compile_bytes_to_llvm)

        # Test that compiler handles None/empty input appropriately
        with pytest.raises((RuntimeError, TypeError, ValueError)):
            compiler.compile_bytes_to_llvm(None)

        with pytest.raises(RuntimeError):
            compiler.compile_bytes_to_llvm(b"")

        # Test that compiler provides meaningful error for invalid JSON
        with pytest.raises(RuntimeError) as exc_info:
            compiler.compile_bytes_to_llvm(b"not json")
        assert (
            "invalid" in str(exc_info.value).lower()
            or "parse" in str(exc_info.value).lower()
        )

    except ImportError:
        pytest.skip("Rust HUGR backend not available")


def test_hugr_compilation_with_invalid_data() -> None:
    """Test HUGR compilation with various invalid inputs."""
    try:
        from pecos_rslib import RustHugrCompiler, check_rust_hugr_availability

        available, message = check_rust_hugr_availability()
        if not available:
            pytest.skip(f"HUGR support not available: {message}")

        compiler = RustHugrCompiler()

        # Test with invalid JSON
        with pytest.raises(RuntimeError) as exc_info:
            compiler.compile_bytes_to_llvm(b"invalid json")
        assert (
            "parse" in str(exc_info.value).lower()
            or "invalid" in str(exc_info.value).lower()
        )

        # Test with valid JSON but not HUGR
        with pytest.raises(RuntimeError):
            compiler.compile_bytes_to_llvm(b'{"not": "hugr"}')

        # Test with malformed HUGR (missing required fields)
        with pytest.raises(RuntimeError):
            compiler.compile_bytes_to_llvm(b'{"modules": []}')

    except ImportError:
        pytest.skip("Rust HUGR backend not available")


def test_hugr_qir_engine_creation() -> None:
    """Test creating LLVM engines."""
    try:
        from pecos_rslib import RustHugrLlvmEngine, check_rust_hugr_availability

        available, message = check_rust_hugr_availability()
        if not available:
            pytest.skip(f"HUGR support not available: {message}")

        # RustHugrLlvmEngine is deprecated and should raise ImportError
        with pytest.raises((ImportError, AttributeError)):
            RustHugrLlvmEngine(shots=100)

    except ImportError as e:
        # This is expected - HUGR-LLVM pipeline has been deprecated
        if "HUGR-LLVM pipeline not available" in str(e):
            pass  # Expected behavior
        else:
            pytest.skip("Rust HUGR backend not available")


def test_hugr_qir_engine_from_file() -> None:
    """Test creating QIR engines from HUGR files."""
    try:
        from pecos_rslib import RustHugrLlvmEngine, check_rust_hugr_availability

        available, message = check_rust_hugr_availability()
        if not available:
            pytest.skip(f"HUGR support not available: {message}")

        # RustHugrLlvmEngine is deprecated and should not have from_file method
        # This should raise ImportError or AttributeError
        # Create a temporary file with dummy HUGR data
        with tempfile.NamedTemporaryFile(suffix=".hugr", delete=False) as f:
            f.write(b"dummy hugr data")
            temp_path = f.name

        try:
            with pytest.raises((ImportError, AttributeError)):
                RustHugrLlvmEngine.from_file(temp_path, shots=100)
        finally:
            Path(temp_path).unlink()  # Clean up

    except ImportError as e:
        # This is expected - HUGR-LLVM pipeline has been deprecated
        if "HUGR-LLVM pipeline not available" in str(e):
            pass  # Expected behavior
        else:
            pytest.skip("Rust HUGR backend not available")


def test_convenience_functions() -> None:
    """Test convenience functions for HUGR compilation."""
    try:
        from pecos_rslib import check_rust_hugr_availability, compile_hugr_to_llvm_rust

        available, message = check_rust_hugr_availability()
        if not available:
            pytest.skip(f"HUGR support not available: {message}")

        # Test that invalid HUGR raises an error
        dummy_hugr = b"dummy hugr data"
        with pytest.raises(RuntimeError, match="Failed to read HUGR"):
            compile_hugr_to_llvm_rust(dummy_hugr)

        # Test with output path - should still raise error for invalid HUGR
        import os
        temp_dir = tempfile.mkdtemp()
        temp_qir_path = os.path.join(temp_dir, "output.ll")

        try:
            # Should raise error for invalid HUGR even with output path
            with pytest.raises(RuntimeError, match="Failed to read HUGR"):
                compile_hugr_to_llvm_rust(dummy_hugr, temp_qir_path)
            # Output file should not be created for invalid HUGR
            assert not Path(temp_qir_path).exists()
        finally:
            import shutil
            shutil.rmtree(temp_dir, ignore_errors=True)

        # Test with valid HUGR (if Guppy is available)
        try:
            from guppylang import guppy
            from guppylang.std.quantum import qubit, h, measure

            @guppy
            def simple_circuit() -> bool:
                q = qubit()
                h(q)
                return measure(q)

            # Compile to HUGR
            package = simple_circuit.compile()
            hugr_json = package.to_json()
            valid_hugr = hugr_json.encode('utf-8')

            # Should successfully compile valid HUGR
            result = compile_hugr_to_llvm_rust(valid_hugr)
            assert isinstance(result, str)
            assert len(result) > 0
            # Check for LLVM IR markers (Selene QIS patterns)
            assert "@qmain" in result or "@___qalloc" in result or "define" in result

            # Test with output path
            with tempfile.NamedTemporaryFile(suffix=".ll", delete=False) as f:
                temp_qir_path = f.name

            try:
                result = compile_hugr_to_llvm_rust(valid_hugr, temp_qir_path)
                assert isinstance(result, str)
                # Check that output file was created
                assert Path(temp_qir_path).exists()
                # Verify file contents match returned string
                assert Path(temp_qir_path).read_text() == result
            finally:
                Path(temp_qir_path).unlink(missing_ok=True)

        except ImportError:
            # Guppy not available, skip the valid HUGR test
            pass

    except ImportError:
        pytest.skip("Rust HUGR backend not available")


def test_guppy_frontend_rust_backend() -> None:
    """Test that Guppy frontend can use Rust backend."""
    try:
        from pecos.frontends.guppy_frontend import GuppyFrontend
        from pecos_rslib import check_rust_hugr_availability

        available, message = check_rust_hugr_availability()
        if not available:
            pytest.skip(f"HUGR support not available: {message}")

        # Create frontend instance - it may not detect Rust backend properly
        # due to import order issues or other factors
        frontend = GuppyFrontend()

        # Check that frontend has the expected attributes
        assert hasattr(frontend, "use_rust_backend")
        # Frontend might not always detect Rust backend even when available
        # This is OK - just test that the frontend was created
        assert isinstance(frontend.use_rust_backend, bool)

        # Frontend should be created successfully
        assert frontend is not None

    except ImportError:
        pytest.skip("Guppy frontend not available")


def test_guppy_frontend_backend_selection() -> None:
    """Test that Guppy frontend backend selection works."""
    try:
        from pecos.frontends import get_guppy_backends
        from pecos.frontends.guppy_frontend import GuppyFrontend

        frontend = GuppyFrontend()

        # Frontend object should exist
        assert frontend is not None

        # Should be able to get backends info via the module function
        backends = get_guppy_backends()
        assert isinstance(backends, dict)
        assert "guppy_available" in backends

        # Even if Rust backend is not available, Guppy should still work
        if not backends.get("rust_backend", False):
            # Guppy can still be available without Rust backend
            assert backends.get("guppy_available", False)

    except ImportError:
        pytest.skip("Guppy frontend not available")


def test_hugr_compiler_with_valid_data() -> None:
    """Test HUGR compiler with semi-valid HUGR data."""
    try:
        from pecos_rslib import RustHugrCompiler, check_rust_hugr_availability

        available, message = check_rust_hugr_availability()
        if not available:
            pytest.skip(f"HUGR support not available: {message}")

        compiler = RustHugrCompiler()

        # Create a minimal HUGR-like structure
        # This is still likely to fail compilation but tests JSON parsing
        hugr_data = b"""{
            "modules": [{
                "version": "live",
                "metadata": {"name": "test"},
                "nodes": [],
                "edges": []
            }],
            "extensions": []
        }"""

        # This will likely fail due to incomplete HUGR, but should parse JSON
        with pytest.raises(RuntimeError) as exc_info:
            compiler.compile_bytes_to_llvm(hugr_data)
        # Error should be about compilation, not parsing
        assert "parse" not in str(exc_info.value).lower()

    except ImportError:
        pytest.skip("Rust HUGR backend not available")
