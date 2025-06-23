#!/usr/bin/env python3
"""Test LLVM IR generation for both conventions to see the actual function calls."""

import tempfile
import os
from pathlib import Path

def test_llvm_generation():
    """Generate LLVM IR for both conventions and inspect the output"""
    print("=" * 60)
    print("Testing LLVM IR Generation")
    print("=" * 60)
    
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
        print(f"HUGR size: {len(hugr_bytes)} bytes")
        
        # Test HUGR convention
        print("\n" + "=" * 40)
        print("HUGR Convention LLVM IR:")
        print("=" * 40)
        
        try:
            hugr_llvm = compile_hugr_to_llvm(hugr_bytes, llvm_convention='hugr')
            
            # Save to temp file and examine
            with tempfile.NamedTemporaryFile(mode='w', suffix='.ll', delete=False) as f:
                f.write(hugr_llvm)
                hugr_file = f.name
            
            # Look for function calls in the HUGR output
            hugr_lines = hugr_llvm.split('\n')
            print("Function calls found in HUGR LLVM IR:")
            for i, line in enumerate(hugr_lines):
                if '__quantum__qis__' in line:
                    print(f"  Line {i+1}: {line.strip()}")
            
            # Check for __hugr suffixed functions
            if '__hugr' in hugr_llvm:
                print("✓ Found __hugr suffixed functions")
            else:
                print("✗ No __hugr suffixed functions found")
                
        except Exception as e:
            print(f"HUGR compilation failed: {e}")
        
        # Test QIR convention  
        print("\n" + "=" * 40)
        print("QIR Convention LLVM IR:")
        print("=" * 40)
        
        try:
            qir_llvm = compile_hugr_to_llvm(hugr_bytes, llvm_convention='qir')
            
            # Save to temp file and examine
            with tempfile.NamedTemporaryFile(mode='w', suffix='.ll', delete=False) as f:
                f.write(qir_llvm)
                qir_file = f.name
            
            # Look for function calls in the QIR output
            qir_lines = qir_llvm.split('\n')
            print("Function calls found in QIR LLVM IR:")
            for i, line in enumerate(qir_lines):
                if '__quantum__qis__' in line:
                    print(f"  Line {i+1}: {line.strip()}")
            
            # Check for opaque pointer types
            if '%Qubit*' in qir_llvm:
                print("✓ Found QIR opaque pointer types (%Qubit*)")
            else:
                print("✗ No QIR opaque pointer types found")
                
        except Exception as e:
            print(f"QIR compilation failed: {e}")
        
        print("\n" + "=" * 60)
        print("LLVM IR Generation Test Complete")
        print("=" * 60)
        
    except Exception as e:
        print(f"Test failed: {e}")

if __name__ == "__main__":
    test_llvm_generation()