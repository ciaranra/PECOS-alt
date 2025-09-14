"""Basic tests for Selene-Guppy compilation pipeline.

This test file focuses on testing the compilation aspects without
requiring the full PECOS engine infrastructure.
"""

import logging

import pytest

# Skip entire module if dependencies aren't available
pytest.importorskip("guppylang")
pytest.importorskip("selene_sim")

from guppylang import guppy
from guppylang.std.quantum import h, measure, qubit

# Set up logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


@pytest.mark.optional_dependency
def test_guppy_to_hugr_compilation() -> None:
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

        logger.info("Compiled Guppy to HUGR: %s bytes", len(hugr_bytes))

    except ImportError as e:
        pytest.skip(f"Compilation pipeline not available: {e}")


@pytest.mark.optional_dependency
def test_selene_builder_import() -> None:
    """Test that SeleneEngineBuilder can be imported and initialized."""
    try:
        from pecos.engines.selene_engine_builder import SeleneEngineBuilder

        # Create a builder instance
        builder = SeleneEngineBuilder(num_qubits=2)
        assert builder is not None
        assert builder.num_qubits == 2

        logger.info("SeleneEngineBuilder imported and initialized")

    except ImportError as e:
        pytest.skip(f"SeleneEngineBuilder not available: {e}")


@pytest.mark.optional_dependency
def test_selene_library_engine_import() -> None:
    """Test that SeleneLibraryEngine can be imported from Rust."""
    try:
        from pecos_rslib import SeleneLibraryEngine

        # Check that the class exists and has expected methods
        assert hasattr(SeleneLibraryEngine, "num_qubits")
        assert hasattr(SeleneLibraryEngine, "reset")

        logger.info("SeleneLibraryEngine imported: %s", SeleneLibraryEngine)

    except ImportError as e:
        pytest.skip(f"SeleneLibraryEngine not available: {e}")


@pytest.mark.optional_dependency
def test_complete_import_chain() -> None:
    """Test that all components of the pipeline can be imported."""
    imports_ok = []
    imports_failed = []

    # Test each import
    import importlib.util

    if importlib.util.find_spec("guppylang") is not None:
        imports_ok.append("guppylang")
    else:
        imports_failed.append("guppylang: not found")

    import importlib.util

    if importlib.util.find_spec("selene_sim") is not None:
        imports_ok.append("selene_sim")
    else:
        imports_failed.append("selene_sim: not found")

    import importlib.util

    if importlib.util.find_spec("pecos.compilation_pipeline") is not None:
        imports_ok.append("compilation_pipeline")
    else:
        imports_failed.append("compilation_pipeline: not found")

    if importlib.util.find_spec("pecos.engines.selene_engine_builder") is not None:
        imports_ok.append("SeleneEngineBuilder")
    else:
        imports_failed.append("SeleneEngineBuilder: not found")

    import importlib.util

    if importlib.util.find_spec("pecos_rslib") is not None:
        try:
            # Check if SeleneLibraryEngine exists in module
            import pecos_rslib

            if hasattr(pecos_rslib, "SeleneLibraryEngine"):
                imports_ok.append("SeleneLibraryEngine")
            else:
                imports_failed.append("SeleneLibraryEngine: not found in pecos_rslib")
        except ImportError:
            imports_failed.append("SeleneLibraryEngine: not in pecos_rslib")
    else:
        imports_failed.append("pecos_rslib: not found")

    # Report results
    logger.info("Successful imports: %s", ", ".join(imports_ok))
    if imports_failed:
        logger.warning("Failed imports: %s", "; ".join(imports_failed))

    # At minimum we need guppylang and selene_sim
    assert "guppylang" in imports_ok
    assert "selene_sim" in imports_ok
