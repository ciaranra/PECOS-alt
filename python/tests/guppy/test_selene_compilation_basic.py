"""Basic tests for Selene-Guppy compilation pipeline.

This test file focuses on testing the compilation aspects without
requiring the full PECOS engine infrastructure.
"""

import pytest
import logging

# Set up logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Skip entire module if dependencies aren't available
pytest.importorskip("guppylang")
pytest.importorskip("selene_sim")

from guppylang import guppy
from guppylang.std.quantum import qubit, h, cx, measure


@pytest.mark.optional_dependency
def test_guppy_to_hugr_compilation():
    """Test that we can compile a Guppy program to HUGR."""
    
    @guppy
    def simple_circuit() -> bool:
        """Apply Hadamard and measure."""
        q = qubit()
        h(q)
        return measure(q)
    
    # Try to import and use the compilation pipeline
    try:
        from pecos.compilation_pipeline import compile_guppy_to_hugr
        
        hugr_bytes = compile_guppy_to_hugr(simple_circuit)
        assert hugr_bytes is not None
        assert len(hugr_bytes) > 0
        
        logger.info(f"✓ Compiled Guppy to HUGR: {len(hugr_bytes)} bytes")
        
    except ImportError as e:
        pytest.skip(f"Compilation pipeline not available: {e}")


@pytest.mark.optional_dependency
def test_selene_builder_import():
    """Test that SeleneEngineBuilder can be imported and initialized."""
    
    try:
        from pecos.engines.selene_engine_builder import SeleneEngineBuilder
        
        # Create a builder instance
        builder = SeleneEngineBuilder(num_qubits=2)
        assert builder is not None
        assert builder.num_qubits == 2
        
        logger.info("✓ SeleneEngineBuilder imported and initialized")
        
    except ImportError as e:
        pytest.skip(f"SeleneEngineBuilder not available: {e}")


@pytest.mark.optional_dependency 
def test_selene_library_engine_import():
    """Test that SeleneLibraryEngine can be imported from Rust."""
    
    try:
        from pecos_rslib import SeleneLibraryEngine
        
        # Check that the class exists and has expected methods
        assert hasattr(SeleneLibraryEngine, 'num_qubits')
        assert hasattr(SeleneLibraryEngine, 'reset')
        
        logger.info(f"✓ SeleneLibraryEngine imported: {SeleneLibraryEngine}")
        
    except ImportError as e:
        pytest.skip(f"SeleneLibraryEngine not available: {e}")


@pytest.mark.optional_dependency
def test_complete_import_chain():
    """Test that all components of the pipeline can be imported."""
    
    imports_ok = []
    imports_failed = []
    
    # Test each import
    try:
        from guppylang import guppy
        imports_ok.append("guppylang")
    except ImportError as e:
        imports_failed.append(f"guppylang: {e}")
    
    try:
        from selene_sim import build, SeleneInstance
        imports_ok.append("selene_sim")
    except ImportError as e:
        imports_failed.append(f"selene_sim: {e}")
    
    try:
        from pecos.compilation_pipeline import compile_guppy_to_hugr
        imports_ok.append("compilation_pipeline")
    except ImportError as e:
        imports_failed.append(f"compilation_pipeline: {e}")
    
    try:
        from pecos.engines.selene_engine_builder import SeleneEngineBuilder
        imports_ok.append("SeleneEngineBuilder")
    except ImportError as e:
        imports_failed.append(f"SeleneEngineBuilder: {e}")
    
    try:
        from pecos_rslib import SeleneLibraryEngine
        imports_ok.append("SeleneLibraryEngine")
    except ImportError as e:
        imports_failed.append(f"SeleneLibraryEngine: {e}")
    
    # Report results
    logger.info(f"✓ Successful imports: {', '.join(imports_ok)}")
    if imports_failed:
        logger.warning(f"✗ Failed imports: {'; '.join(imports_failed)}")
    
    # At minimum we need guppylang and selene_sim
    assert "guppylang" in imports_ok
    assert "selene_sim" in imports_ok


if __name__ == "__main__":
    # Run tests for debugging
    print("=" * 60)
    print("SELENE COMPILATION BASIC TESTS")
    print("=" * 60)
    
    test_guppy_to_hugr_compilation()
    test_selene_builder_import()
    test_selene_library_engine_import()
    test_complete_import_chain()
    
    print("\nAll tests completed!")