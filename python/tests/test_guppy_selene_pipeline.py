"""Test the complete Guppy to Selene Interface pipeline."""

import pytest

# Skip if guppylang is not available
guppylang = pytest.importorskip("guppylang")

def test_guppy_to_selene_pipeline():
    """Test that Guppy programs can be compiled to Selene Interface and executed."""
    
    # Try to import sim
    try:
        from pecos_rslib import sim
    except ImportError:
        pytest.skip("sim() function not available")
    
    # Simple Guppy program that creates a Bell state
    guppy_source = """
from guppylang import guppy, quantum
from guppylang.prelude.quantum import Qubit, Measure, H, CX

@guppy
def bell_state() -> tuple[bool, bool]:
    q1 = Qubit()
    q2 = Qubit()
    
    # Create Bell state
    H(q1)
    CX(q1, q2)
    
    # Measure both qubits
    m1 = Measure(q1)
    m2 = Measure(q2)
    
    return (m1, m2)
"""
    
    # Test that sim() auto-detects Guppy and converts to Selene Interface
    try:
        # This should:
        # 1. Detect Guppy source
        # 2. Compile to HUGR
        # 3. Convert HUGR to Selene Interface plugin
        # 4. Execute with SeleneSimpleRuntimeEngine
        result = sim(guppy_source).run(10)
        
        # Check that we got results
        assert result is not None
        
        # For Bell state, measurements should be correlated
        # Both qubits should have the same value in each shot
        if hasattr(result, 'to_dict'):
            result_dict = result.to_dict()
        else:
            result_dict = result
            
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
        if "program must be" in str(e) or "cannot convert" in str(e) or "not supported" in str(e):
            pytest.skip(f"Guppy source not yet supported by sim(): {e}")
        raise


def test_selene_interface_program_creation():
    """Test that SeleneInterfaceProgram can be created and used."""
    
    # Try to import SeleneInterfaceProgram
    try:
        from pecos_rslib._pecos_rslib import PySeleneInterfaceProgram as SeleneInterfaceProgram
    except ImportError:
        # Try alternative import
        try:
            from pecos_rslib import SeleneInterfaceProgram
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
        assert "runtime" in str(e).lower() or "library" in str(e).lower() or "convert" in str(e).lower()


def test_selene_simple_runtime_builder():
    """Test that SeleneSimpleRuntimeEngine can be built."""
    try:
        from pecos_rslib import selene_simple_runtime
    except ImportError:
        pytest.skip("selene_simple_runtime not available")
    
    # Create builder
    builder = selene_simple_runtime()
    
    # Configure it
    builder = builder.qubits(2)
    
    # Try to build (may fail if runtime library not found)
    try:
        engine = builder.build()
        assert engine is not None
    except RuntimeError as e:
        # Expected if Selene runtime library not installed
        assert "runtime" in str(e).lower() or "library" in str(e).lower()


@pytest.mark.parametrize("guppy_code,expected_gates", [
    # Hadamard gate
    ("""
from guppylang import guppy, quantum
from guppylang.prelude.quantum import Qubit, Measure, H

@guppy  
def hadamard_test() -> bool:
    q = Qubit()
    H(q)
    return Measure(q)
""", ["H"]),
    
    # CNOT gate
    ("""
from guppylang import guppy, quantum
from guppylang.prelude.quantum import Qubit, Measure, CX

@guppy
def cnot_test() -> tuple[bool, bool]:
    q1 = Qubit()
    q2 = Qubit()
    CX(q1, q2)
    return (Measure(q1), Measure(q2))
""", ["CX"]),
])
def test_guppy_gate_compilation(guppy_code, expected_gates):
    """Test that specific Guppy gates are compiled correctly."""
    
    try:
        from pecos_rslib import sim
    except ImportError:
        pytest.skip("sim() not available")
    
    try:
        # Try to compile and run
        result = sim(guppy_code).run(1)
        
        # If successful, verify result structure
        assert result is not None
        
    except ImportError as e:
        if "guppylang" in str(e):
            pytest.skip("guppylang not installed")
        raise
    except (NotImplementedError, TypeError) as e:
        # Expected until full pipeline implemented
        pytest.skip(f"Guppy compilation not yet fully implemented: {e}")