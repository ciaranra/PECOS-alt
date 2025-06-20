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
    # Create a HUGR that looks like what Guppy would generate
    hugr = {
        "version": "0.1.0",
        "name": "random_bit",
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
    }
    
    hugr_json = json.dumps(hugr)
    
    # Convert HUGR to PAST (PECOS AST)
    past_ron = hugr_to_past_ron(hugr_json)
    assert past_ron.startswith("(")
    assert "AllocQubit" in past_ron
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
    # Create a Bell state HUGR
    hugr = {
        "version": "0.1.0",
        "name": "bell_state",
        "nodes": [
            {"op": {"type": "AllocQubit"}},  # Node 0
            {"op": {"type": "AllocQubit"}},  # Node 1
            {"op": {"type": "H"}},           # Node 2
            {"op": {"type": "CX"}},          # Node 3
            {"op": {"type": "Measure"}},     # Node 4
            {"op": {"type": "Measure"}},     # Node 5
            {"op": {"type": "Output", "port": 0}},  # Node 6
            {"op": {"type": "Output", "port": 1}}   # Node 7
        ],
        "edges": [
            {"src": [0, 0], "dst": [2, 0]},  # Qubit 0 -> H
            {"src": [2, 0], "dst": [3, 0]},  # H -> CX control
            {"src": [1, 0], "dst": [3, 1]},  # Qubit 1 -> CX target
            {"src": [3, 0], "dst": [4, 0]},  # CX control -> Measure
            {"src": [3, 1], "dst": [5, 0]},  # CX target -> Measure
            {"src": [4, 0], "dst": [6, 0]},  # Measure -> Output
            {"src": [5, 0], "dst": [7, 0]}   # Measure -> Output
        ]
    }
    
    hugr_json = json.dumps(hugr)
    
    # Convert to PMIR
    pmir_mlir = hugr_to_pmir_mlir(hugr_json, debug_output=False, optimization_level=2)
    
    # Verify PMIR contains expected operations
    assert "func @main" in pmir_mlir
    assert "call @__quantum__rt__qubit_allocate" in pmir_mlir
    assert "call @__quantum__qis__h__body" in pmir_mlir
    assert "call @__quantum__qis__cnot__body" in pmir_mlir
    assert "call @__quantum__qis__mz__body" in pmir_mlir
    assert "return" in pmir_mlir


@pytest.mark.skipif(not PMIR_AVAILABLE, reason="PMIR not available")
def test_pmir_with_manual_hugr():
    """Test PMIR with a manually created HUGR (no Guppy dependency)."""
    # Create a simple HUGR manually
    hugr = {
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