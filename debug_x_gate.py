#!/usr/bin/env python3
"""Debug X gate compilation."""

import sys
import tempfile
sys.path.append('/home/ciaranra/Repos/cl_projects/gup/PECOS/python/quantum-pecos/src')

try:
    import guppy
    from pecos.frontends.run_guppy import run_guppy
    
    @guppy.guppy
    def simple_x_test() -> bool:
        """Simple X gate test."""
        q = guppy.qubit()
        guppy.x(q)  # Apply X gate
        return guppy.measure(q)
    
    print("=== Compiling simple X gate test ===")
    
    # Try to run it and see what happens
    results = run_guppy(simple_x_test, shots=10, seed=42)
    print("Results:", results)
    
    # Also check what QIR is generated
    if "hugr_llvm" in results and results["hugr_llvm"].get("success"):
        qir_file = results["hugr_llvm"]["result"].get("qir_file")
        if qir_file:
            print(f"\n=== Generated QIR file: {qir_file} ===")
            with open(qir_file, 'r') as f:
                qir_content = f.read()
                print(qir_content)
                
                # Check if X gate is in the QIR
                if "__quantum__qis__x__body" in qir_content:
                    print("\n✓ X gate found in QIR")
                else:
                    print("\n✗ X gate NOT found in QIR!")
                    print("Available quantum operations:")
                    for line in qir_content.split('\n'):
                        if "__quantum__qis__" in line:
                            print(f"  {line.strip()}")
            
except Exception as e:
    print(f"Error: {e}")
    import traceback
    traceback.print_exc()