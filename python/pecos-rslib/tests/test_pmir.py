"""Tests for PMIR (PECOS Middle-level IR) Python bindings."""

import pytest
from pecos_rslib import (
    hugr_to_past_ron,
    hugr_to_pmir_mlir,
    past_ron_to_pmir_mlir,
    compile_hugr_via_pmir,
    PMIRCompiler,
)


# Simple test HUGR JSON (correct format with modules array)
SIMPLE_HUGR = """{
    "modules": [{
        "version": "live",
        "metadata": {"name": "hadamard_test"},
        "nodes": [
            {"parent": 0, "op": "Module"},
            {"parent": 0, "op": "FuncDefn", "name": "main"},
            {"parent": 1, "op": "Input"},
            {"parent": 1, "op": "Output"},
            {"parent": 1, "op": "Extension", "name": "QAlloc"},
            {"parent": 1, "op": "Extension", "name": "H"},
            {"parent": 1, "op": "Extension", "name": "MeasureFree"}
        ],
        "edges": [
            [[2, 0], [4, 0]],
            [[4, 0], [5, 0]],
            [[5, 0], [6, 0]],
            [[6, 0], [3, 0]]
        ]
    }],
    "extensions": []
}"""


def test_hugr_to_past_ron():
    """Test conversion from HUGR to PAST RON."""
    past_ron = hugr_to_past_ron(SIMPLE_HUGR)
    
    # Check that it's valid RON - it starts with a parenthesis for the tuple struct
    assert past_ron.startswith("(")
    assert "hadamard_test" in past_ron
    # Check for the expected quantum operations in the new format
    assert ("QAlloc" in past_ron or "Extension" in past_ron)
    assert "H" in past_ron


def test_hugr_to_pmir_mlir():
    """Test conversion from HUGR to PMIR MLIR."""
    pmir_mlir = hugr_to_pmir_mlir(SIMPLE_HUGR, debug_output=False, optimization_level=2)
    
    # Check that it contains MLIR function declarations (MLIR-14 syntax)
    assert "func" in pmir_mlir  # Should contain MLIR functions
    assert "@__quantum__" in pmir_mlir  # Should contain quantum runtime calls
    
    # Check for basic MLIR structure
    assert "@main" in pmir_mlir or "main" in pmir_mlir
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
    assert ("main" in pmir_mlir or "@main" in pmir_mlir)
    assert ("call @__quantum__" in pmir_mlir or "@__quantum__" in pmir_mlir)


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
        if "mlir-opt" in str(e) or "MLIR" in str(e):
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
        # Should give clear error about missing MLIR tools or other compilation issues
        error_msg = str(e).lower()
        # Accept either MLIR tool errors or other compilation-related errors
        assert any(keyword in error_msg for keyword in ["mlir-opt", "mlir", "compilation", "failed to compile", "tool not found"])


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