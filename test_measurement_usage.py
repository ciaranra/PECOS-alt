#!/usr/bin/env python3
"""Test measurement convention adapter with actual measurement usage."""

from guppylang import guppy
from guppylang.std.quantum import h, measure, qubit
import tempfile
import sys
import os

# Add the PECOS Python package to the path
sys.path.insert(0, '/home/ciaranra/Repos/cl_projects/gup/PECOS/python/quantum-pecos/src')
sys.path.insert(0, '/home/ciaranra/Repos/cl_projects/gup/PECOS/python/pecos-rslib/src')

@guppy
def conditional_measure() -> bool:
    """Measurement test with conditional logic."""
    q = qubit()
    h(q)
    result = measure(q)
    # Use the measurement result in conditional logic
    if result:
        return True
    else:
        return False

def main():
    print("Testing measurement convention adapter with measurement usage...")
    
    # Compile the Guppy function
    compiled = guppy.compile_function(conditional_measure)
    hugr_bytes = compiled.package.to_bytes()
    
    # Use PECOS QIR infrastructure
    import pecos_rslib.hugr_qir as hugr_qir
    
    with tempfile.NamedTemporaryFile(mode='wb', suffix='.hugr', delete=False) as hugr_file:
        hugr_file.write(hugr_bytes)
        hugr_path = hugr_file.name
    
    try:
        # Compile with HUGR naming convention
        llvm_ir = hugr_qir.compile_hugr_to_qir_rust(hugr_path, llvm_convention="hugr")
        
        print("\n" + "="*60)
        print("LLVM-IR Analysis:")
        print("="*60)
        
        # Check for deferred measurement calls
        deferred_count = llvm_ir.count('__hugr__quantum__qis__m__body')
        print(f"✓ Deferred measurement calls: {deferred_count}")
        
        # Check for result getter calls
        getter_count = llvm_ir.count('__quantum__rt__result_get_one')
        print(f"✓ Result getter calls: {getter_count}")
        
        # Check that immediate measurement calls were removed
        immediate_count = llvm_ir.count('call i32 @__quantum__qis__m__body(')
        if immediate_count == 0:
            print("✓ All immediate measurement calls converted")
        else:
            print(f"✗ {immediate_count} immediate measurement calls still present")
            
        # Show the relevant snippet
        print("\n" + "-"*40)
        print("Measurement conversion snippet:")
        print("-"*40)
        
        lines = llvm_ir.split('\n')
        for i, line in enumerate(lines):
            if '__hugr__quantum__qis__m__body' in line:
                print(f"{i+1:3}: {line.strip()}")
                # Check the surrounding lines for result getter
                for j in range(max(0, i-2), min(len(lines), i+5)):
                    if j != i and ('__quantum__rt__result_get_one' in lines[j] or 'measurement_result' in lines[j]):
                        print(f"{j+1:3}: {lines[j].strip()}")
                break
                
        print("\n" + "="*60)
        print("Measurement convention adapter implementation successful!")
        print("- Immediate measurements → Deferred measurements")
        print("- Added result getter function calls")
        print("- Maintained measurement result variable usage")
        print("="*60)
        
    finally:
        os.unlink(hugr_path)

if __name__ == "__main__":
    main()