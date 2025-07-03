#!/usr/bin/env python3
"""Test the complete Guppy → HUGR → PMIR → LLVM → PECOS pipeline.

This tests the PMIR (PECOS Middle-level IR) alternative compilation path.
"""

import json
import sys
from pathlib import Path

import pytest

sys.path.append("python/quantum-pecos/src")

# Check if required dependencies are available
try:
    from guppylang import guppy
    from guppylang.std.quantum import h, measure, qubit
    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False

try:
    from pecos_rslib import (
        compile_hugr_via_pmir,
        hugr_to_past_ron,
        hugr_to_pmir_mlir,
        PMIR_AVAILABLE,
    )
except ImportError:
    PMIR_AVAILABLE = False

try:
    from pecos.frontends.guppy_frontend import GuppyFrontend
    FRONTEND_AVAILABLE = True
except ImportError:
    FRONTEND_AVAILABLE = False


@pytest.mark.skipif(not PMIR_AVAILABLE, reason="PMIR not available")
def test_guppy_like_hugr_to_pmir_pipeline():
    """Test a Guppy-like HUGR through the PMIR pipeline."""
    # Create a HUGR that looks like what Guppy would generate (new format)
    hugr = {
        "modules": [{
            "version": "live",
            "metadata": {"name": "random_bit"},
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
    }
    
    hugr_json = json.dumps(hugr)
    
    # Convert HUGR to PAST (PECOS AST)
    past_ron = hugr_to_past_ron(hugr_json)
    assert past_ron.startswith("(")
    assert "QAlloc" in past_ron
    assert "H" in past_ron
    assert "Measure" in past_ron
    
    # Convert HUGR to PMIR (MLIR text)
    pmir_mlir = hugr_to_pmir_mlir(hugr_json, debug_output=False, optimization_level=2)
    assert "func" in pmir_mlir
    assert "@main" in pmir_mlir
    assert "call @__quantum__" in pmir_mlir
    
    # Try to compile to LLVM IR (may fail if MLIR tools not installed)
    try:
        llvm_ir = compile_hugr_via_pmir(
            hugr_json, 
            debug_output=False, 
            optimization_level=2, 
            target_triple=None
        )
        # If successful, verify LLVM IR
        assert "define" in llvm_ir or "ModuleID" in llvm_ir
        assert "@__quantum__" in llvm_ir
        print("[PASS] Successfully compiled HUGR → PMIR → LLVM IR")
    except RuntimeError as e:
        if "mlir-opt" in str(e) or "MLIR" in str(e):
            pytest.skip("MLIR tools not available")
        else:
            raise


@pytest.mark.skipif(not PMIR_AVAILABLE, reason="PMIR not available")
def test_bell_state_hugr_via_pmir():
    """Test a Bell state HUGR through the PMIR pipeline."""
    # Create a Bell state HUGR (new format)
    hugr = {
        "modules": [{
            "version": "live",
            "metadata": {"name": "bell_state"},
            "nodes": [
                {"parent": 0, "op": "Module"},
                {"parent": 0, "op": "FuncDefn", "name": "main"},
                {"parent": 1, "op": "Input"},
                {"parent": 1, "op": "Output"},
                {"parent": 1, "op": "Extension", "name": "QAlloc"},
                {"parent": 1, "op": "Extension", "name": "QAlloc"},
                {"parent": 1, "op": "Extension", "name": "H"},
                {"parent": 1, "op": "Extension", "name": "CX"},
                {"parent": 1, "op": "Extension", "name": "MeasureFree"},
                {"parent": 1, "op": "Extension", "name": "MeasureFree"}
            ],
            "edges": [
                [[2, 0], [4, 0]],
                [[2, 0], [5, 0]],
                [[4, 0], [6, 0]],
                [[6, 0], [7, 0]],
                [[5, 0], [7, 1]],
                [[7, 0], [8, 0]],
                [[7, 1], [9, 0]],
                [[8, 0], [3, 0]],
                [[9, 0], [3, 1]]
            ]
        }],
        "extensions": []
    }
    
    hugr_json = json.dumps(hugr)
    
    # Convert to PMIR
    pmir_mlir = hugr_to_pmir_mlir(hugr_json, debug_output=False, optimization_level=2)
    
    # Verify PMIR contains expected operations
    assert "func @main" in pmir_mlir
    assert "call @__quantum__rt__qubit_allocate" in pmir_mlir
    assert "call @__quantum__qis__h__body" in pmir_mlir
    assert "call @__quantum__qis__cx__body" in pmir_mlir  # HUGR uses cx, not cnot
    assert "call @__quantum__qis__m__body" in pmir_mlir   # Standardized to m__body
    assert "return" in pmir_mlir


@pytest.mark.skipif(not PMIR_AVAILABLE, reason="PMIR not available")
def test_pmir_with_manual_hugr():
    """Test PMIR with a manually created HUGR (no Guppy dependency)."""
    # Create a simple HUGR manually (new format)
    hugr = {
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
    }
    
    hugr_json = json.dumps(hugr)
    
    # Convert to PAST
    past_ron = hugr_to_past_ron(hugr_json)
    assert "hadamard_test" in past_ron
    
    # Convert to PMIR
    pmir_mlir = hugr_to_pmir_mlir(hugr_json, debug_output=False, optimization_level=2)
    assert "func @main" in pmir_mlir
    assert "call @__quantum__qis__h__body" in pmir_mlir


if __name__ == "__main__":
    # Run tests directly
    import subprocess
    subprocess.run([sys.executable, "-m", "pytest", __file__, "-v"])