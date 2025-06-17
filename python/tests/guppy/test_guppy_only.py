"""
Guppy-only tests that don't require full PECOS installation.
These tests focus just on the Guppy integration components.
"""

import sys
from pathlib import Path

# Add PECOS to path
PECOS_ROOT = Path(__file__).parent.parent.parent.parent
sys.path.insert(0, str(PECOS_ROOT / "python" / "quantum-pecos" / "src"))


def test_guppy_frontend_imports():
    """Test that Guppy frontend modules can be imported"""
    # Import just the guppy frontend modules without importing all of PECOS
    from pecos.frontends.guppy_frontend import GuppyFrontend
    from pecos.frontends.run_guppy import get_guppy_backends
    from pecos.frontends.qir_engine_wrapper import QirEngineWrapper
    
    # If we get here, imports worked
    assert True


def test_guppy_available():
    """Test if Guppy is available in the environment"""
    try:
        import guppylang
        from guppylang import guppy
        
        @guppy
        def test_func(x: int) -> int:
            return x + 1
        
        # Function should be decorated
        assert hasattr(test_func, '__wrapped__') or hasattr(test_func, 'name')
        print("✅ Guppy is available and working")
        
    except ImportError:
        import pytest
        pytest.skip("guppylang not available - install with: uv pip install guppylang")


def test_backend_detection_minimal():
    """Test backend detection without full PECOS"""
    from pecos.frontends.run_guppy import get_guppy_backends
    
    backends = get_guppy_backends()
    
    # Should return a dict
    assert isinstance(backends, dict)
    assert 'guppy_available' in backends
    assert 'rust_backend' in backends
    
    print(f"Guppy available: {backends['guppy_available']}")
    print(f"Rust backend: {backends['rust_backend']}")


if __name__ == "__main__":
    """Run tests directly"""
    print("🧪 Running Guppy-only tests...")
    
    test_guppy_frontend_imports()
    print("✅ Frontend imports work")
    
    test_backend_detection_minimal()
    print("✅ Backend detection works")
    
    test_guppy_available()
    
    print("✅ All Guppy tests passed!")