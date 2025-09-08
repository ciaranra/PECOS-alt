"""
Tests for HUGR/LLVM PyO3 integration

Tests the Rust backend for HUGR compilation and LLVM engine creation.
"""

import pytest
import tempfile
from pathlib import Path


# Test availability checks
def test_hugr_backend_availability() -> None:
    """Test that we can check HUGR backend availability."""
    try:
        from pecos_rslib import check_rust_hugr_availability, RUST_HUGR_AVAILABLE

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
            "json" in str(exc_info.value).lower()
            or "parse" in str(exc_info.value).lower()
        )

    except ImportError:
        pytest.skip("Rust HUGR backend not available")


def test_hugr_compilation_with_invalid_data() -> None:
    """Test that compilation fails gracefully with invalid HUGR data."""
    try:
        from pecos_rslib import RustHugrCompiler, check_rust_hugr_availability

        available, message = check_rust_hugr_availability()
        if not available:
            pytest.skip(f"HUGR support not available: {message}")

        # HUGR support is available - test error handling with invalid data
        compiler = RustHugrCompiler()

        # Create some invalid HUGR bytes
        invalid_hugr = b"this is not valid HUGR data"

        with pytest.raises(RuntimeError) as exc_info:
            compiler.compile_bytes_to_llvm(invalid_hugr)

        # Verify we get a reasonable error message
        error_msg = str(exc_info.value).lower()
        assert any(
            keyword in error_msg for keyword in ["json", "parse", "hugr", "invalid"]
        ), f"Expected parsing error, got: {exc_info.value}"

    except ImportError:
        pytest.skip("Rust HUGR backend not available")


def test_hugr_qir_engine_creation() -> None:
    """Test creating QIR engines from HUGR data."""
    try:
        from pecos_rslib import RustHugrLlvmEngine, check_rust_hugr_availability

        available, message = check_rust_hugr_availability()
        if not available:
            pytest.skip(f"HUGR support not available: {message}")

        # Create some dummy HUGR bytes
        dummy_hugr = b"dummy hugr data"

        # This will likely fail due to invalid HUGR data, but tests the interface
        with pytest.raises(RuntimeError):
            RustHugrLlvmEngine(dummy_hugr, shots=100)

    except ImportError:
        pytest.skip("Rust HUGR backend not available")


def test_hugr_qir_engine_from_file() -> None:
    """Test creating QIR engines from HUGR files."""
    try:
        from pecos_rslib import RustHugrLlvmEngine, check_rust_hugr_availability

        available, message = check_rust_hugr_availability()
        if not available:
            pytest.skip(f"HUGR support not available: {message}")

        # Create a temporary file with dummy HUGR data
        with tempfile.NamedTemporaryFile(suffix=".hugr", delete=False) as f:
            f.write(b"dummy hugr data")
            temp_path = f.name

        try:
            # This will likely fail due to invalid HUGR data, but tests the interface
            with pytest.raises(RuntimeError):
                RustHugrLlvmEngine.from_file(temp_path, shots=100)
        finally:
            Path(temp_path).unlink()  # Clean up

    except ImportError:
        pytest.skip("Rust HUGR backend not available")


def test_convenience_functions() -> None:
    """Test convenience functions for HUGR compilation."""
    try:
        from pecos_rslib import compile_hugr_to_llvm_rust, check_rust_hugr_availability

        available, message = check_rust_hugr_availability()
        if not available:
            pytest.skip(f"HUGR support not available: {message}")

        # Test with bytes (should fail due to invalid HUGR data)
        dummy_hugr = b"dummy hugr data"
        with pytest.raises(RuntimeError):
            compile_hugr_to_llvm_rust(dummy_hugr)

        # Test with file path
        with tempfile.NamedTemporaryFile(suffix=".hugr", delete=False) as f:
            f.write(dummy_hugr)
            temp_hugr_path = f.name

        with tempfile.NamedTemporaryFile(suffix=".ll", delete=False) as f:
            temp_qir_path = f.name

        try:
            with pytest.raises(RuntimeError):
                compile_hugr_to_llvm_rust(temp_hugr_path, temp_qir_path)
        finally:
            Path(temp_hugr_path).unlink()
            Path(temp_qir_path).unlink()

    except ImportError:
        pytest.skip("Rust HUGR backend not available")


def test_guppy_frontend_rust_backend() -> None:
    """Test GuppyFrontend with Rust backend."""
    try:
        from pecos.frontends import GuppyFrontend

        # Test creating frontend with Rust backend preference
        frontend = GuppyFrontend(use_rust_backend=True)
        backend_info = frontend.get_backend_info()

        assert "backend" in backend_info
        assert "rust_available" in backend_info
        assert "guppy_available" in backend_info

        # If Rust backend is available, it should be used
        if backend_info["rust_available"]:
            assert backend_info["backend"] == "rust"

    except ImportError as e:
        if "guppylang" in str(e):
            pytest.skip("Guppylang not available")
        elif "Rust backend" in str(e):
            pytest.skip("Rust backend not available")
        else:
            raise


def test_guppy_frontend_backend_selection() -> None:
    """Test backend selection logic in GuppyFrontend."""
    try:
        from pecos.frontends import GuppyFrontend

        # Test auto-detection
        frontend = GuppyFrontend()
        info = frontend.get_backend_info()

        # Should auto-select best available backend
        if info["rust_available"]:
            assert info["backend"] == "rust"
        else:
            assert info["backend"] == "external"

        # Test forcing external backend
        frontend = GuppyFrontend(use_rust_backend=False)
        info = frontend.get_backend_info()
        assert info["backend"] == "external"

    except ImportError as e:
        if "guppylang" in str(e):
            pytest.skip("Guppylang not available")
        else:
            raise


def test_hugr_compiler_with_valid_data() -> None:
    """Test HUGR compiler with valid HUGR JSON data."""
    try:
        from pecos_rslib import RustHugrCompiler
        import json

        compiler = RustHugrCompiler()

        # Create a minimal valid HUGR structure
        # This represents an empty module with proper HUGR format
        valid_hugr = {
            "format": "hugr",
            "version": "0.1.0",
            "modules": [
                {
                    "name": "main",
                    "nodes": [{"id": "root", "op": "Module", "children": []}],
                    "edges": [],
                }
            ],
        }

        hugr_bytes = json.dumps(valid_hugr).encode("utf-8")

        # This should either compile successfully or fail with a specific HUGR error
        # (not a JSON parsing error)
        try:
            result = compiler.compile_bytes_to_llvm(hugr_bytes)
            # If it succeeds, verify we got LLVM IR
            assert isinstance(result, str)
            assert len(result) > 0
            # Basic LLVM IR should contain some expected patterns
            assert "define" in result or "declare" in result or "@" in result
        except RuntimeError as e:
            # If it fails, it should be due to HUGR validation, not JSON parsing
            error_msg = str(e).lower()
            assert (
                "json" not in error_msg or "hugr" in error_msg
            ), f"Expected HUGR validation error, not JSON parsing error: {e}"

    except ImportError:
        pytest.skip("Rust HUGR backend not available")


if __name__ == "__main__":
    # Run tests if this file is executed directly
    pytest.main([__file__])
