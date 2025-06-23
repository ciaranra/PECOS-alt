#!/usr/bin/env python3
"""Debug LLVM IR generation to check entry points and function names."""

import tempfile
import os

def debug_llvm_ir():
    """Generate and examine LLVM IR for both conventions."""
    try:
        from guppylang import guppy, qubit
        from guppylang.std.quantum import h
        from pecos.compilation_pipeline import compile_guppy_to_hugr, compile_hugr_to_llvm
        
        @guppy
        def simple_h() -> qubit:
            q = qubit()
            h(q)
            return q
        
        print("Compiling to HUGR...")
        hugr_bytes = compile_guppy_to_hugr(simple_h)
        
        # Generate HUGR convention LLVM IR
        print("\n" + "="*50)
        print("HUGR Convention LLVM IR:")
        print("="*50)
        hugr_llvm = compile_hugr_to_llvm(hugr_bytes, llvm_convention='hugr')
        
        # Save to file and display
        with tempfile.NamedTemporaryFile(mode='w', suffix='.ll', delete=False) as f:
            f.write(hugr_llvm)
            hugr_file = f.name
        
        print(f"Generated file: {hugr_file}")
        print("\nFull LLVM IR content:")
        print(hugr_llvm)
        
        # Look for entry point attribute
        if 'EntryPoint' in hugr_llvm:
            print("\n✓ Found EntryPoint attribute")
        else:
            print("\n✗ NO EntryPoint attribute found!")
        
        # Look for function definitions
        print("\nFunction definitions:")
        for i, line in enumerate(hugr_llvm.split('\n')):
            if 'define ' in line:
                print(f"  Line {i+1}: {line}")
        
        # Look for attributes sections
        print("\nAttributes sections:")
        for i, line in enumerate(hugr_llvm.split('\n')):
            if 'attributes ' in line:
                print(f"  Line {i+1}: {line}")
        
        print("\n" + "="*50)
        print("QIR Convention LLVM IR:")
        print("="*50)
        qir_llvm = compile_hugr_to_llvm(hugr_bytes, llvm_convention='qir')
        
        # Save to file
        with tempfile.NamedTemporaryFile(mode='w', suffix='.ll', delete=False) as f:
            f.write(qir_llvm)
            qir_file = f.name
        
        print(f"Generated file: {qir_file}")
        print("\nFull LLVM IR content:")
        print(qir_llvm)
        
        # Look for entry point attribute
        if 'EntryPoint' in qir_llvm:
            print("\n✓ Found EntryPoint attribute")
        else:
            print("\n✗ NO EntryPoint attribute found!")
        
        # Look for function definitions
        print("\nFunction definitions:")
        for i, line in enumerate(qir_llvm.split('\n')):
            if 'define ' in line:
                print(f"  Line {i+1}: {line}")
        
        # Look for attributes sections
        print("\nAttributes sections:")
        for i, line in enumerate(qir_llvm.split('\n')):
            if 'attributes ' in line:
                print(f"  Line {i+1}: {line}")
        
        print(f"\nFiles saved:")
        print(f"HUGR: {hugr_file}")
        print(f"QIR: {qir_file}")
        
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    debug_llvm_ir()