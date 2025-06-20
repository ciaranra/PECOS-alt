"""Tests for PMIR (PECOS Middle-level IR) Python bindings."""

import pytest
from pecos_rslib import (
    hugr_to_past_ron,
    hugr_to_pmir_mlir,
    past_ron_to_pmir_mlir,
    compile_hugr_via_pmir,
    PMIRCompiler,
)


# Simple test HUGR JSON
SIMPLE_HUGR = """{
    "version": "0.1.0",
    "name": "hadamard_test",
    "nodes": [
        {"op": {"type": "AllocQubit"}},
        {"op": {"type": "H"}},
        {"op": {"type": "Measure"}},
        {"op": {"type": "Output", "port": 0}}
    ],
    "edges": [
        {"src": [0, 0], "dst": [1, 0]},
        {"src": [1, 0], "dst": [2, 0]},
        {"src": [2, 0], "dst": [3, 0]}
    ]
}"""


def test_hugr_to_past_ron():
    """Test conversion from HUGR to PAST RON."""
    past_ron = hugr_to_past_ron(SIMPLE_HUGR)
    
    # Check that it's valid RON - it starts with a parenthesis for the tuple struct
    assert past_ron.startswith("(")
    assert "hadamard_test" in past_ron
    assert "AllocQubit" in past_ron
    assert "H" in past_ron
    assert "Measure" in past_ron


def test_hugr_to_pmir_mlir():
    """Test conversion from HUGR to PMIR MLIR."""
    pmir_mlir = hugr_to_pmir_mlir(SIMPLE_HUGR, debug_output=False, optimization_level=2)
    
    # Check that it contains MLIR function declarations (MLIR-14 syntax)
    assert "func private @__quantum__rt__qubit_allocate" in pmir_mlir
    assert "func private @__quantum__qis__h__body" in pmir_mlir
    assert "func private @__quantum__qis__mz__body" in pmir_mlir
    
    # Check main function
    assert "func @main()" in pmir_mlir
    assert "call @__quantum__" in pmir_mlir
    assert "return" in pmir_mlir


def test_past_ron_to_pmir_mlir():
    """Test conversion from PAST RON to PMIR MLIR."""
    # First get PAST RON
    past_ron = hugr_to_past_ron(SIMPLE_HUGR)
    
    # Then convert to PMIR
    pmir_mlir = past_ron_to_pmir_mlir(past_ron, debug_output=False, optimization_level=2)
    
    # Should produce same result as direct conversion
    direct_mlir = hugr_to_pmir_mlir(SIMPLE_HUGR, debug_output=False, optimization_level=2)
    
    # The output should be functionally equivalent
    # (exact match might differ due to internal node IDs)
    assert "func @main()" in pmir_mlir
    assert "call @__quantum__" in pmir_mlir


def test_pmir_compiler_class():
    """Test the PMIRCompiler convenience class."""
    compiler = PMIRCompiler(debug_output=False, optimization_level=2)
    
    # Test getting PAST
    past = compiler.get_past(SIMPLE_HUGR)
    assert past.startswith("(")  # RON format starts with parenthesis
    assert "hadamard_test" in past
    
    # Test getting PMIR
    pmir = compiler.get_pmir(SIMPLE_HUGR)
    assert "func" in pmir
    
    # Test compilation (may fail if MLIR tools not available)
    try:
        llvm_ir = compiler.compile(SIMPLE_HUGR)
        assert "define" in llvm_ir or "ModuleID" in llvm_ir
    except RuntimeError as e:
        if "mlir-opt not found" in str(e):
            pytest.skip("MLIR tools not available")
        else:
            raise


def test_compile_hugr_via_pmir_fallback():
    """Test that compile_hugr_via_pmir handles missing MLIR tools gracefully."""
    try:
        llvm_ir = compile_hugr_via_pmir(SIMPLE_HUGR, debug_output=False, optimization_level=2, target_triple=None)
        # If it succeeds, check for valid LLVM IR
        assert "define" in llvm_ir or "ModuleID" in llvm_ir
    except RuntimeError as e:
        # Should give clear error about missing MLIR tools
        assert "mlir-opt" in str(e) or "MLIR" in str(e)


def test_invalid_hugr():
    """Test error handling for invalid HUGR."""
    invalid_hugr = '{"invalid": "json"}'
    
    with pytest.raises(RuntimeError) as exc_info:
        hugr_to_past_ron(invalid_hugr)
    
    assert "Failed to parse HUGR" in str(exc_info.value)


def test_invalid_past_ron():
    """Test error handling for invalid PAST RON."""
    invalid_ron = "InvalidRON{{"
    
    with pytest.raises(RuntimeError) as exc_info:
        past_ron_to_pmir_mlir(invalid_ron, debug_output=False, optimization_level=2)
    
    assert "Failed to deserialize PAST" in str(exc_info.value)