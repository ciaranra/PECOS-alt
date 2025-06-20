#!/usr/bin/env python3
"""
Example demonstrating the PMIR (PECOS Middle-level IR) compilation pipeline.

This shows how to use the alternative compilation path from HUGR to LLVM IR
via MLIR infrastructure.
"""

import json
from pecos_rslib import (
    hugr_to_past_ron,
    hugr_to_pmir_mlir,
    compile_hugr_via_pmir,
    compile_and_execute_via_pmir,
    PMIRCompiler,
)


def create_bell_state_hugr():
    """Create a simple Bell state circuit in HUGR format."""
    return {
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


def main():
    print("PMIR (PECOS Middle-level IR) Compilation Pipeline Example")
    print("=" * 60)
    
    # Create Bell state circuit
    hugr = create_bell_state_hugr()
    hugr_json = json.dumps(hugr, indent=2)
    
    print("\n1. Original HUGR JSON:")
    print(hugr_json)
    
    # Convert to PAST (PECOS AST) in RON format
    print("\n2. Converting HUGR to PAST (RON format)...")
    past_ron = hugr_to_past_ron(hugr_json)
    print("PAST RON (first 500 chars):")
    print(past_ron[:500] + "..." if len(past_ron) > 500 else past_ron)
    
    # Convert to PMIR (MLIR text)
    print("\n3. Converting HUGR to PMIR (MLIR text)...")
    pmir_mlir = hugr_to_pmir_mlir(hugr_json, debug_output=True, optimization_level=2)
    print("PMIR as MLIR:")
    print(pmir_mlir)
    
    # Try to compile to LLVM IR (requires MLIR tools)
    print("\n4. Attempting to compile to LLVM IR via MLIR tools...")
    try:
        llvm_ir = compile_hugr_via_pmir(hugr_json, debug_output=True, optimization_level=2, target_triple=None)
        print("Success! Generated LLVM IR (first 1000 chars):")
        print(llvm_ir[:1000] + "..." if len(llvm_ir) > 1000 else llvm_ir)
    except RuntimeError as e:
        print(f"Note: Compilation failed - {e}")
        print("This is expected if MLIR tools (mlir-opt, mlir-translate) are not installed.")
        print("The PMIR generation still works and produces valid MLIR text.")
    
    # Demonstrate the high-level compiler interface
    print("\n5. Using PMIRCompiler convenience class...")
    compiler = PMIRCompiler(debug_output=False, optimization_level=2)
    
    # Get intermediate representations
    past = compiler.get_past(hugr_json)
    pmir = compiler.get_pmir(hugr_json)
    
    print(f"PAST size: {len(past)} characters")
    print(f"PMIR size: {len(pmir)} characters")
    
    # Try execution (if compilation works)
    print("\n6. Attempting execution via PMIR pipeline...")
    try:
        results = compile_and_execute_via_pmir(hugr_json, 10, False, 2)
        print(f"Executed {len(results)} shots:")
        for i, result in enumerate(results):
            print(f"  Shot {i+1}: {result}")
    except (RuntimeError, NotImplementedError) as e:
        print(f"Note: Execution failed - {e}")
        print("This is expected - execution via PMIR is not yet implemented.")
    
    print("\n" + "=" * 60)
    print("Summary:")
    print("- HUGR → PAST conversion: ✓ Working")
    print("- PAST → PMIR (MLIR) generation: ✓ Working")
    print("- PMIR → LLVM IR compilation: Requires MLIR tools")
    print("- The PMIR pipeline provides an alternative compilation path")
    print("- It leverages MLIR infrastructure for optimization and lowering")


if __name__ == "__main__":
    main()