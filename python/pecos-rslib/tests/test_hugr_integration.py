"""
Tests for HUGR/QIR PyO3 integration

Tests the Rust backend for HUGR compilation and QIR engine creation.
"""

import pytest
import tempfile
from pathlib import Path

# Test availability checks
def test_hugr_backend_availability():
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


def test_hugr_compiler_creation():
    """Test creating HUGR compiler instances."""
    try:
        from pecos_rslib import RustHugrCompiler
        
        # Test default creation
        compiler = RustHugrCompiler()
        assert compiler.get_naming_convention() == "standard"
        
        # Test with parameters
        compiler = RustHugrCompiler(debug_info=True, naming_convention="hugr")
        assert compiler.get_naming_convention() == "hugr"
        
        # Test setting naming convention
        compiler.set_naming_convention("pecos")
        assert compiler.get_naming_convention() == "pecos"
        
        # Test invalid naming convention
        with pytest.raises(Exception):  # Should raise ValueError
            compiler.set_naming_convention("invalid")
            
    except ImportError:
        pytest.skip("Rust HUGR backend not available")


def test_supported_naming_conventions():
    """Test getting supported naming conventions."""
    try:
        from pecos_rslib import RustHugrCompiler
        
        conventions = RustHugrCompiler.get_supported_naming_conventions()
        assert isinstance(conventions, list)
        assert len(conventions) > 0
        assert "standard" in conventions
        
    except ImportError:
        pytest.skip("Rust HUGR backend not available")


def test_hugr_compilation_without_hugr_support():
    """Test that compilation fails gracefully when HUGR support is not compiled in."""
    try:
        from pecos_rslib import RustHugrCompiler, check_rust_hugr_availability
        
        available, message = check_rust_hugr_availability()
        if not available and "not compiled" in message:
            # HUGR support not compiled in - test should fail gracefully
            compiler = RustHugrCompiler()
            
            # Create some dummy HUGR bytes
            dummy_hugr = b"dummy hugr data"
            
            with pytest.raises(Exception):  # Should raise RuntimeError
                compiler.compile_bytes_to_qir(dummy_hugr)
        else:
            pytest.skip("HUGR support is available, skipping negative test")
            
    except ImportError:
        pytest.skip("Rust HUGR backend not available")


def test_hugr_qir_engine_creation():
    """Test creating QIR engines from HUGR data."""
    try:
        from pecos_rslib import RustHugrQirEngine, check_rust_hugr_availability
        
        available, message = check_rust_hugr_availability()
        if not available:
            pytest.skip(f"HUGR support not available: {message}")
        
        # Create some dummy HUGR bytes
        dummy_hugr = b"dummy hugr data"
        
        # This will likely fail due to invalid HUGR data, but tests the interface
        with pytest.raises(Exception):  # Should raise RuntimeError due to invalid HUGR
            engine = RustHugrQirEngine(dummy_hugr, shots=100)
            
    except ImportError:
        pytest.skip("Rust HUGR backend not available")


def test_hugr_qir_engine_from_file():
    """Test creating QIR engines from HUGR files."""
    try:
        from pecos_rslib import RustHugrQirEngine, check_rust_hugr_availability
        
        available, message = check_rust_hugr_availability()
        if not available:
            pytest.skip(f"HUGR support not available: {message}")
        
        # Create a temporary file with dummy HUGR data
        with tempfile.NamedTemporaryFile(suffix='.hugr', delete=False) as f:
            f.write(b"dummy hugr data")
            temp_path = f.name
        
        try:
            # This will likely fail due to invalid HUGR data, but tests the interface
            with pytest.raises(Exception):  # Should raise RuntimeError due to invalid HUGR
                engine = RustHugrQirEngine.from_file(temp_path, shots=100)
        finally:
            Path(temp_path).unlink()  # Clean up
            
    except ImportError:
        pytest.skip("Rust HUGR backend not available")


def test_convenience_functions():
    """Test convenience functions for HUGR compilation."""
    try:
        from pecos_rslib import compile_hugr_to_qir_rust, check_rust_hugr_availability
        
        available, message = check_rust_hugr_availability()
        if not available:
            pytest.skip(f"HUGR support not available: {message}")
        
        # Test with bytes (should fail due to invalid HUGR data)
        dummy_hugr = b"dummy hugr data"
        with pytest.raises(Exception):  # Should raise RuntimeError
            qir = compile_hugr_to_qir_rust(dummy_hugr)
            
        # Test with file path
        with tempfile.NamedTemporaryFile(suffix='.hugr', delete=False) as f:
            f.write(dummy_hugr)
            temp_hugr_path = f.name
        
        with tempfile.NamedTemporaryFile(suffix='.ll', delete=False) as f:
            temp_qir_path = f.name
        
        try:
            with pytest.raises(Exception):  # Should raise RuntimeError
                compile_hugr_to_qir_rust(temp_hugr_path, temp_qir_path)
        finally:
            Path(temp_hugr_path).unlink()
            Path(temp_qir_path).unlink()
            
    except ImportError:
        pytest.skip("Rust HUGR backend not available")


def test_guppy_frontend_rust_backend():
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


def test_guppy_frontend_backend_selection():
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


if __name__ == "__main__":
    # Run tests if this file is executed directly
    pytest.main([__file__])