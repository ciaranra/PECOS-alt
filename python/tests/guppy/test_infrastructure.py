"""
Basic infrastructure tests for Guppy integration.
These are pytest-compatible tests.
"""

import sys
from pathlib import Path

# Add PECOS to path
PECOS_ROOT = Path(__file__).parent.parent.parent.parent
sys.path.insert(0, str(PECOS_ROOT / "python" / "quantum-pecos" / "src"))


def test_python_imports():
    """Test that basic Python imports work"""
    from pecos.frontends.run_guppy import get_guppy_backends
    from pecos.frontends.guppy_frontend import GuppyFrontend
    
    # If we get here, imports worked
    assert True


def test_backend_detection():
    """Test backend detection functionality"""
    from pecos.frontends.run_guppy import get_guppy_backends
    
    backends = get_guppy_backends()
    
    # Should return a dict with the expected keys
    assert isinstance(backends, dict)
    assert 'guppy_available' in backends
    assert 'rust_backend' in backends
    assert 'external_tools' in backends
    
    # These should be boolean values
    assert isinstance(backends['guppy_available'], bool)
    assert isinstance(backends['rust_backend'], bool)
    assert isinstance(backends['external_tools'], bool)


def test_guppy_frontend_creation():
    """Test that GuppyFrontend can be created"""
    from pecos.frontends.guppy_frontend import GuppyFrontend
    
    frontend = GuppyFrontend()
    
    # Should be able to get backend info
    info = frontend.get_backend_info()
    assert isinstance(info, dict)
    assert 'backend' in info
    
    # Clean up
    frontend.cleanup()


def test_guppy_import_if_available():
    """Test Guppy import if available (may be skipped)"""
    try:
        import guppylang
        from guppylang import guppy
        
        # If we get here, guppylang is available
        @guppy
        def simple_func(x: int) -> int:
            return x + 1
        
        # Function should be decorated (check for guppy-specific attributes)
        assert hasattr(simple_func, 'wrapped') or str(type(simple_func)).startswith("<class 'guppylang")
        
    except ImportError:
        # Guppy not available, skip this test
        import pytest
        pytest.skip("guppylang not available")