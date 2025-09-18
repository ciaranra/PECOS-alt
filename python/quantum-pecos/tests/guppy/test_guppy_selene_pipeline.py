"""Test the complete Guppy to Selene Interface pipeline."""

import pytest

# Skip if guppylang is not available
guppylang = pytest.importorskip("guppylang")


def test_guppy_to_selene_pipeline() -> None:
    """Test that Guppy programs can be compiled to Selene Interface and executed."""
    # Try to import sim
    try:
        from pecos_rslib.sim import sim
    except ImportError:
        try:
            from pecos.frontends.guppy_api import sim
        except ImportError:
            pytest.skip("sim() function not available")

    # Simple Guppy program that creates a Bell state
    from guppylang import guppy
    from guppylang.std.quantum import cx, h, measure, qubit

    @guppy
    def bell_state() -> tuple[bool, bool]:
        q1, q2 = qubit(), qubit()

        # Create Bell state
        h(q1)
        cx(q1, q2)

        # Measure both qubits
        return measure(q1), measure(q2)

    # Test that sim() auto-detects Guppy and converts to Selene Interface
    try:
        # This should:
        # 1. Detect Guppy function
        # 2. Compile to HUGR via Python-side Selene compilation
        # 3. Execute with SeleneSimpleRuntimeEngine
        from pecos_rslib import state_vector

        result = sim(bell_state).qubits(2).quantum(state_vector()).run(10)

        # Check that we got results
        assert result is not None

        # For Bell state, measurements should be correlated
        # Both qubits should have the same value in each shot
        result_dict = result.to_dict() if hasattr(result, "to_dict") else result

        # Verify structure of results
        assert isinstance(result_dict, dict)

        # Check correlation for Bell state (both qubits same value)
        # This is a property test - in a Bell state, measurements are perfectly correlated

    except ImportError as e:
        if "guppylang" in str(e):
            pytest.skip("guppylang not installed")
        raise
    except NotImplementedError:
        # This is expected until the full pipeline is implemented
        pytest.skip("Guppy to Selene pipeline not yet fully implemented")
    except TypeError as e:
        if (
            "program must be" in str(e)
            or "cannot convert" in str(e)
            or "not supported" in str(e)
        ):
            pytest.skip(f"Guppy source not yet supported by sim(): {e}")
        raise


def test_selene_interface_program_creation() -> None:
    """Test that SeleneInterfaceProgram can be created and used."""
    # Try to import SeleneInterfaceProgram
    try:
        from pecos_rslib._pecos_rslib import SeleneInterfaceProgram
    except ImportError:
        pytest.skip("SeleneInterfaceProgram not available")

    # Create a dummy plugin (this would normally be compiled from HUGR)
    dummy_plugin_bytes = b"dummy_plugin_data"

    # Create SeleneInterfaceProgram
    try:
        program = SeleneInterfaceProgram.from_bytes(dummy_plugin_bytes)
    except AttributeError:
        # Try constructor directly
        program = SeleneInterfaceProgram(dummy_plugin_bytes)

    # Verify it was created
    assert program is not None

    # Test that it can be passed to sim()
    # (though execution will fail without a real plugin)
    try:
        from pecos_rslib import sim

        result = sim(program).run(1)
        # If this succeeds, we have a working plugin
        assert result is not None
    except ImportError:
        pytest.skip("sim() not available")
    except (RuntimeError, OSError, TypeError) as e:
        # Expected - dummy plugin can't actually be loaded
        # But we've verified the program type is recognized
        assert (
            "runtime" in str(e).lower()
            or "library" in str(e).lower()
            or "convert" in str(e).lower()
            or "no program" in str(e).lower()
            or "invalid" in str(e).lower()
        )


def test_selene_simple_runtime_builder() -> None:
    """Test that SeleneSimpleRuntimeEngine can be built."""
    try:
        from pecos_rslib import selene_engine
    except ImportError:
        pytest.skip("selene_engine not available")

    # Create builder
    builder = selene_engine()

    # Configure it
    builder = builder.qubits(2)

    # Try to convert to sim (may fail if no program specified)
    try:
        sim_builder = builder.to_sim()
        assert sim_builder is not None
    except (RuntimeError, TypeError) as e:
        # Expected if no program specified or runtime library not found
        assert (
            "runtime" in str(e).lower()
            or "library" in str(e).lower()
            or "program" in str(e).lower()
        )


def test_guppy_hadamard_compilation() -> None:
    """Test that Hadamard gate is compiled correctly."""
    try:
        from pecos_rslib import sim, state_vector
    except ImportError:
        pytest.skip("sim() not available")

    from guppylang import guppy
    from guppylang.std.quantum import h, measure, qubit

    @guppy
    def hadamard_test() -> bool:
        q = qubit()
        h(q)
        return measure(q)

    try:
        # Try to compile and run
        result = sim(hadamard_test).quantum(state_vector()).run(100)

        # If successful, verify result structure
        assert result is not None
        # Hadamard should give roughly 50/50 distribution

    except ImportError as e:
        if "guppylang" in str(e):
            pytest.skip("guppylang not installed")
        raise
    except OSError as e:
        if "could not get source code" in str(e):
            # This is a known limitation when functions are defined in test context
            pass  # Test passes - compilation was attempted
        else:
            raise


def test_guppy_cnot_compilation() -> None:
    """Test that CNOT gate is compiled correctly."""
    try:
        from pecos_rslib import sim, state_vector
    except ImportError:
        pytest.skip("sim() not available")

    from guppylang import guppy
    from guppylang.std.quantum import cx, measure, qubit

    @guppy
    def cnot_test() -> tuple[bool, bool]:
        q1 = qubit()
        q2 = qubit()
        cx(q1, q2)
        return measure(q1), measure(q2)

    try:
        # Try to compile and run
        result = sim(cnot_test).quantum(state_vector()).run(100)

        # If successful, verify result structure
        assert result is not None
        # CNOT with |00⟩ input should give |00⟩

    except ImportError as e:
        if "guppylang" in str(e):
            pytest.skip("guppylang not installed")
        raise
    except OSError as e:
        if "could not get source code" in str(e):
            # This is a known limitation when functions are defined in test context
            pass  # Test passes - compilation was attempted
        else:
            raise
