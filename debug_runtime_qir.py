#!/usr/bin/env python3
"""Debug what QIR file the runtime is actually trying to execute."""

import tempfile
import os

def debug_runtime_qir():
    """Generate QIR and inspect the actual file the runtime gets."""
    try:
        from guppylang import guppy, qubit
        from guppylang.std.quantum import h
        from pecos.frontends.run_guppy import run_guppy
        
        @guppy
        def simple_h() -> qubit:
            q = qubit()
            h(q)
            return q
        
        print("Testing HUGR convention...")
        print("=" * 50)
        
        # Patch the run_guppy function to intercept the QIR file path
        original_run_guppy = run_guppy
        qir_file_path = None
        
        def patched_execute_qir(file_path, shots, seed, noise_prob, workers, llvm_convention=None):
            global qir_file_path
            qir_file_path = file_path
            
            # Read and print the actual QIR content
            print(f"QIR file path: {file_path}")
            with open(file_path, 'r') as f:
                qir_content = f.read()
            
            print("Actual QIR content being executed:")
            print("=" * 60)
            print(qir_content)
            print("=" * 60)
            
            # Check for EntryPoint
            if 'EntryPoint' in qir_content:
                print("✓ EntryPoint attribute found in runtime QIR")
            else:
                print("✗ NO EntryPoint attribute in runtime QIR!")
            
            # Check function definitions
            print("\nFunction definitions in runtime QIR:")
            for i, line in enumerate(qir_content.split('\n')):
                if 'define ' in line:
                    print(f"  Line {i+1}: {line}")
            
            # Check attributes
            print("\nAttributes in runtime QIR:")
            for i, line in enumerate(qir_content.split('\n')):
                if 'attributes ' in line:
                    print(f"  Line {i+1}: {line}")
            
            # Raise an error to prevent actual execution and just show the debugging info
            raise RuntimeError("Debug mode - stopping before execution")
        
        # Monkey patch for debugging
        import pecos_rslib
        pecos_rslib.execute_qir = patched_execute_qir
        
        try:
            result = run_guppy(simple_h, shots=1, llvm_convention='hugr', verbose=True)
        except RuntimeError as e:
            if "Debug mode" in str(e):
                print(f"\nDebugging complete. QIR file saved at: {qir_file_path}")
            else:
                raise
        
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    debug_runtime_qir()