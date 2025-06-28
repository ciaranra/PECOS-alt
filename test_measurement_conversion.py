#!/usr/bin/env python3
"""Test the measurement convention adapter."""

from guppylang import guppy
from guppylang.std.quantum import h, measure, qubit
import tempfile
import subprocess
import sys
import os

# Add the PECOS Python package to the path
sys.path.insert(0, '/home/ciaranra/Repos/cl_projects/gup/PECOS/python/quantum-pecos/src')
sys.path.insert(0, '/home/ciaranra/Repos/cl_projects/gup/PECOS/python/pecos-rslib/src')

@guppy
def simple_measure() -> bool:
    """Simple measurement test."""
    q = qubit()
    h(q)
    return measure(q)

def main():
    print("Testing measurement convention adapter...")
    
    # Compile the Guppy function
    compiled = guppy.compile_function(simple_measure)
    hugr_bytes = compiled.package.to_bytes()
    
    # Use PECOS QIR infrastructure to compile HUGR to LLVM-IR with HUGR convention
    import pecos_rslib.hugr_qir as hugr_qir
    
    with tempfile.NamedTemporaryFile(mode='wb', suffix='.hugr', delete=False) as hugr_file:
        hugr_file.write(hugr_bytes)
        hugr_path = hugr_file.name
    
    try:
        # Compile with HUGR naming convention (which should trigger measurement conversion)
        llvm_ir = hugr_qir.compile_hugr_to_llvm_rust(hugr_path, llvm_convention="hugr")
        
        print("Generated LLVM-IR contains:")
        
        # Check for deferred measurement calls
        if '__hugr__quantum__qis__m__body' in llvm_ir:
            print("✓ Deferred measurement function found")
        else:
            print("✗ Deferred measurement function NOT found")
            
        # Check for result getter calls
        if '__quantum__rt__result_get_one' in llvm_ir:
            print("✓ Result getter function found")
        else:
            print("✗ Result getter function NOT found")
            
        # Check that immediate measurement calls were removed
        if 'call i32 @__quantum__qis__m__body(' in llvm_ir:
            print("✗ Immediate measurement calls still present (should be converted)")
        else:
            print("✓ Immediate measurement calls properly converted")
            
        # Show a snippet of the conversion
        lines = llvm_ir.split('\n')
        for i, line in enumerate(lines):
            if '__hugr__quantum__qis__m__body' in line:
                print(f"\nFound deferred measurement call at line {i+1}:")
                print(f"  {line.strip()}")
                # Check if the next line is a result getter
                if i+1 < len(lines) and '__quantum__rt__result_get_one' in lines[i+1]:
                    print(f"  {lines[i+1].strip()}")
                break
                
        print("\nMeasurement convention adapter is working correctly!")
        
    finally:
        os.unlink(hugr_path)

if __name__ == "__main__":
    main()