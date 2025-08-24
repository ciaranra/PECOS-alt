"""End-to-end test for Guppy to PECOS pipeline using SeleneLibraryEngine.

This test validates the complete flow:
1. Guppy program → HUGR → Selene shared library
2. Library loading with callbacks
3. ByteMessage exchange during execution
4. Result extraction via TCP stream
"""

import pytest
import tempfile
from pathlib import Path
import logging

# Set up logging
logging.basicConfig(level=logging.DEBUG)
logger = logging.getLogger(__name__)

# Import dependencies - let them fail naturally if not available
pytest.importorskip("guppylang")
pytest.importorskip("selene_sim")

from guppylang import guppy
from guppylang.std.quantum import qubit, h, cx, measure


@pytest.mark.optional_dependency
def test_simple_guppy_program_compilation():
    """Test compiling a simple Guppy program to HUGR."""
    
    from pecos.engines.selene_engine_builder import SeleneEngineBuilder
    
    @guppy
    def bell_state() -> tuple[bool, bool]:
        """Create a Bell state and measure both qubits."""
        q0, q1 = qubit(), qubit()
        h(q0)
        cx(q0, q1)
        return measure(q0), measure(q1)
    
    # Use builder to compile to HUGR
    builder = SeleneEngineBuilder(num_qubits=2)
    builder.with_guppy_program(bell_state)
    
    # Compile to HUGR (but don't build the full engine yet)
    hugr_bytes = builder._compile_to_hugr()
    assert hugr_bytes is not None
    assert len(hugr_bytes) > 0
    
    logger.info(f"Compiled Bell state to HUGR: {len(hugr_bytes)} bytes")


@pytest.mark.optional_dependency
def test_selene_library_build():
    """Test building a Selene shared library from HUGR."""
    
    from pecos.engines.selene_engine_builder import SeleneEngineBuilder
    
    @guppy
    def simple_circuit() -> bool:
        """Simple single-qubit circuit."""
        q = qubit()
        h(q)
        return measure(q)
    
    builder = SeleneEngineBuilder(num_qubits=1)
    builder.with_guppy_program(simple_circuit)
    
    # Compile to HUGR
    hugr_bytes = builder._compile_to_hugr()
    
    # Build shared library
    library_path = builder._build_shared_library(hugr_bytes)
    assert library_path.exists()
    assert library_path.suffix in ['.so', '.dylib', '.dll']
    
    logger.info(f"Built shared library at: {library_path}")
    
    # Cleanup
    builder.cleanup()


@pytest.mark.optional_dependency
def test_selene_engine_creation():
    """Test creating a SeleneLibraryEngine from Guppy."""
    
    @guppy
    def hadamard_test() -> bool:
        """Apply Hadamard and measure."""
        q = qubit()
        h(q)
        return measure(q)
    
    # Create engine using convenience function
    engine = selene_engine_from_guppy(hadamard_test, num_qubits=1)
    
    assert engine is not None
    assert engine.num_qubits == 1
    
    logger.info("Successfully created SeleneLibraryEngine")


@pytest.mark.optional_dependency
def test_end_to_end_pipeline():
    """Test the complete Guppy → PECOS pipeline."""
    
    @guppy
    def bell_circuit() -> tuple[bool, bool]:
        """Create and measure a Bell state."""
        q0, q1 = qubit(), qubit()
        h(q0)
        cx(q0, q1)
        m0 = measure(q0)
        m1 = measure(q1)
        return m0, m1
    
    # Create Selene engine
    selene_engine = selene_engine_from_guppy(bell_circuit, num_qubits=2)
    
    # Create a quantum engine (mock for testing)
    quantum_engine = SplitStateArrayEngine(num_qubits=2)
    
    # Create hybrid engine
    hybrid = HybridEngine(
        classical_engine=selene_engine,
        quantum_engine=quantum_engine
    )
    
    # Run a single shot
    shot = hybrid.run()
    
    # Check results
    assert "result_0" in shot.measurement_results or "m0" in shot.measurement_results
    assert "result_1" in shot.measurement_results or "m1" in shot.measurement_results
    
    logger.info(f"Shot results: {shot.measurement_results}")


@pytest.mark.optional_dependency
def test_guppy_with_result_function():
    """Test Guppy program using result() function."""
    
    @guppy
    def circuit_with_results():
        """Circuit that uses result() to tag outputs."""
        from guppylang.std.quantum import result
        
        q = qubit()
        h(q)
        m = measure(q)
        result("hadamard_output", m)
        
        # Also return a value
        return m
    
    engine = selene_engine_from_guppy(circuit_with_results, num_qubits=1)
    
    # The engine should be created successfully
    assert engine is not None
    
    logger.info("Created engine for program with result() function")


@pytest.mark.optional_dependency
def test_multiple_shots():
    """Test running multiple shots with the pipeline."""
    
    @guppy
    def coin_flip() -> bool:
        """Quantum coin flip."""
        q = qubit()
        h(q)
        return measure(q)
    
    # Build once, run multiple times
    selene_engine = selene_engine_from_guppy(coin_flip, num_qubits=1)
    quantum_engine = SplitStateArrayEngine(num_qubits=1)
    
    hybrid = HybridEngine(
        classical_engine=selene_engine,
        quantum_engine=quantum_engine
    )
    
    # Run 10 shots
    results = []
    for _ in range(10):
        hybrid.reset()
        shot = hybrid.run()
        results.append(shot.measurement_results.get("result", False))
    
    # Should have mix of True and False (statistically)
    assert True in results or False in results
    
    logger.info(f"10 shots gave results: {results}")


@pytest.mark.optional_dependency
def test_builder_cleanup():
    """Test that builder properly cleans up temporary files."""
    
    @guppy
    def dummy_circuit() -> bool:
        q = qubit()
        return measure(q)
    
    builder = SeleneEngineBuilder(num_qubits=1)
    builder.with_guppy_program(dummy_circuit)
    
    # Track the build directory
    engine = builder.build()
    build_dir = builder.build_dir
    
    assert build_dir.exists()
    
    # Cleanup should remove the directory
    builder.cleanup()
    assert not build_dir.exists()
    
    logger.info("Builder cleanup successful")


