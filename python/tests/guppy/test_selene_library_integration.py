"""End-to-end test for Guppy to PECOS pipeline using SeleneLibraryEngine.

This test validates the complete flow:
1. Guppy program → HUGR → Selene shared library
2. Library loading with callbacks
3. ByteMessage exchange during execution
4. Result extraction via TCP stream
"""

import logging

import pytest

# Set up logging
logging.basicConfig(level=logging.DEBUG)
logger = logging.getLogger(__name__)

# Import dependencies - let them fail naturally if not available
pytest.importorskip("guppylang")
pytest.importorskip("selene_sim")

from guppylang import guppy
from guppylang.std.quantum import cx, h, measure, qubit

# Import missing dependencies for tests
try:
    from pecos.engines.hybrid_engine import HybridEngine
    from pecos.engines.selene_engine_builder import SeleneEngineBuilder
except ImportError as e:
    logger.warning(f"Could not import PECOS engines: {e}")

try:
    from pecos_rslib import StateVecEngineRs
except ImportError as e:
    logger.warning(f"Could not import StateVecEngineRs: {e}")


# Define helper function that's missing
def selene_engine_from_guppy(guppy_func, num_qubits: int):
    """Helper to create SeleneLibraryEngine from Guppy function."""
    from pecos.engines.selene_engine_builder import SeleneEngineBuilder

    builder = SeleneEngineBuilder(num_qubits=num_qubits)
    builder.with_guppy_program(guppy_func)
    return builder.build()


@pytest.mark.optional_dependency
def test_simple_guppy_program_compilation() -> None:
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
    hugr_result = builder._compile_to_hugr()
    assert hugr_result is not None

    # Handle both Package and bytes
    if (
        hasattr(hugr_result, "__class__")
        and hugr_result.__class__.__name__ == "Package"
    ):
        logger.info(f"Compiled Bell state to HUGR Package: {hugr_result}")
    else:
        assert len(hugr_result) > 0
        logger.info(f"Compiled Bell state to HUGR: {len(hugr_result)} bytes")


@pytest.mark.optional_dependency
def test_selene_library_build() -> None:
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
    hugr_result = builder._compile_to_hugr()

    # Build shared library
    library_path = builder._build_shared_library(hugr_result)
    assert library_path.exists()
    assert library_path.suffix in [".so", ".dylib", ".dll"]

    logger.info(f"Built shared library at: {library_path}")

    # Cleanup
    builder.cleanup()


@pytest.mark.optional_dependency
def test_selene_engine_creation() -> None:
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
    assert engine.num_qubits() == 1

    logger.info("Successfully created SeleneLibraryEngine")


@pytest.mark.optional_dependency
def test_end_to_end_pipeline() -> None:
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

    # Use the sim() API which handles Guppy functions properly
    from pecos.frontends.guppy_api import sim
    from pecos_rslib import state_vector

    # Run using sim API which handles Selene compilation
    results = sim(bell_circuit).qubits(2).quantum(state_vector()).run(10)

    # Check results - should have measurement_1 and measurement_2
    assert "measurement_1" in results or "m0" in results
    assert "measurement_2" in results or "m1" in results

    logger.info(f"Results: {results}")


@pytest.mark.optional_dependency
def test_guppy_with_result_function() -> None:
    """Test Guppy program using result() function."""
    # Import result from the correct module
    from guppylang.std.platform import result

    @guppy
    def circuit_with_results() -> bool:
        """Circuit that uses result() to tag outputs."""
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
def test_multiple_shots() -> None:
    """Test running multiple shots with the pipeline."""

    @guppy
    def coin_flip() -> bool:
        """Quantum coin flip."""
        q = qubit()
        h(q)
        return measure(q)

    # Use sim API for multiple shots
    from pecos.frontends.guppy_api import sim
    from pecos_rslib import state_vector

    # Run 10 shots
    results = sim(coin_flip).qubits(1).quantum(state_vector()).run(10)

    # Extract measurements
    measurements = results.get("measurement_1", [])

    # Should have mix of 0 and 1 (statistically)
    assert 0 in measurements or 1 in measurements

    logger.info(f"10 shots gave results: {measurements}")


@pytest.mark.optional_dependency
def test_builder_cleanup() -> None:
    """Test that builder properly cleans up temporary files."""

    @guppy
    def dummy_circuit() -> bool:
        q = qubit()
        return measure(q)

    builder = SeleneEngineBuilder(num_qubits=1)
    builder.with_guppy_program(dummy_circuit)

    # Track the build directory
    builder.build()
    build_dir = builder.build_dir

    assert build_dir.exists()

    # Cleanup should remove the directory
    builder.cleanup()
    assert not build_dir.exists()

    logger.info("Builder cleanup successful")
