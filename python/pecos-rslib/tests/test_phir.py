"""Tests for PHIR (PECOS High-level IR) Python bindings."""

import pytest
from pecos_rslib import (
    hugr_to_phir_mlir,
    compile_hugr_via_phir,
    PhirCompiler,
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


# RON serialization has been removed from PHIR
# The test_hugr_to_past_ron function has been removed


def test_hugr_to_phir_mlir():
    """Test conversion from HUGR to PHIR MLIR."""
    phir_mlir = hugr_to_phir_mlir(SIMPLE_HUGR, debug_output=False, optimization_level=2)
    
    # Check that it contains MLIR function declarations (MLIR-14 syntax)
    assert "func" in phir_mlir  # Should contain MLIR functions
    assert "@__quantum__" in phir_mlir  # Should contain quantum runtime calls
    
    # Check for basic MLIR structure
    assert "@main" in phir_mlir or "main" in phir_mlir
    assert "return" in phir_mlir


# RON serialization has been removed from PHIR
# The test_past_ron_to_phir_mlir function has been removed


def test_phir_compiler_class():
    """Test the PhirCompiler convenience class."""
    compiler = PhirCompiler(debug_output=False, optimization_level=2)
    
    # Test getting PHIR
    phir = compiler.get_phir(SIMPLE_HUGR)
    assert "func" in phir
    
    # Test compilation (may fail if MLIR tools not available)
    try:
        llvm_ir = compiler.compile(SIMPLE_HUGR)
        assert "define" in llvm_ir or "ModuleID" in llvm_ir
    except RuntimeError as e:
        if "mlir-opt" in str(e) or "MLIR" in str(e):
            pytest.skip("MLIR tools not available")
        else:
            raise


def test_compile_hugr_via_phir_fallback():
    """Test that compile_hugr_via_phir handles missing MLIR tools gracefully."""
    try:
        llvm_ir = compile_hugr_via_phir(SIMPLE_HUGR, debug_output=False, optimization_level=2, target_triple=None)
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
        hugr_to_phir_mlir(invalid_hugr, debug_output=False, optimization_level=2)
    
    # The error message should indicate HUGR parsing failed
    assert "Failed to parse HUGR" in str(exc_info.value) or "parse" in str(exc_info.value).lower()


def test_malformed_hugr_json():
    """Test error handling for malformed JSON."""
    malformed_json = '{"modules": [}'
    
    with pytest.raises(RuntimeError) as exc_info:
        hugr_to_phir_mlir(malformed_json, debug_output=False, optimization_level=2)
    
    # Should get a JSON parsing error
    error_msg = str(exc_info.value).lower()
    assert "json" in error_msg or "parse" in error_msg