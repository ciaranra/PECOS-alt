#!/usr/bin/env python3
"""Debug what HUGR is generated for conditional operations"""

import sys
import tempfile
import os
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import qubit, measure, x
from pecos.frontends import guppy_sim
from pecos.compilation_pipeline import compile_guppy_to_hugr

@guppy
def test_conditional() -> bool:
    """Simple conditional to examine HUGR"""
    q1 = qubit()
    x(q1)  # q1 = |1⟩
    
    q2 = qubit()  # q2 = |0⟩
    
    if measure(q1):  # Should be True
        x(q2)  # Should execute
    
    return measure(q2)

if __name__ == "__main__":
    print("Generating HUGR for conditional operation...")
    try:
        # Compile to HUGR
        hugr_bytes = compile_guppy_to_hugr(test_conditional)
        
        # Save HUGR to file for inspection
        with tempfile.NamedTemporaryFile(mode='wb', suffix='.hugr', delete=False) as f:
            f.write(hugr_bytes)
            hugr_path = f.name
        
        print(f"HUGR saved to: {hugr_path}")
        print(f"HUGR size: {len(hugr_bytes)} bytes")
        
        # Try to examine first few hundred bytes as text (might be JSON)
        try:
            text_preview = hugr_bytes[:500].decode('utf-8', errors='ignore')
            print(f"HUGR preview: {text_preview[:200]}...")
        except:
            print("HUGR appears to be binary format")
        
        # Try to compile to LLVM to see where it fails
        print("\nAttempting LLVM compilation...")
        from pecos.compilation_pipeline import compile_hugr_to_llvm
        
        llvm_ir = compile_hugr_to_llvm(hugr_bytes)
        print("LLVM compilation succeeded!")
        
        # Save LLVM IR to file
        with open('/tmp/test_conditional.ll', 'w') as f:
            f.write(llvm_ir)
        print(f"LLVM IR saved to: /tmp/test_conditional.ll")
        print(f"LLVM IR size: {len(llvm_ir)} characters")
        
        # Show first few lines of LLVM IR
        lines = llvm_ir.split('\n')[:10]
        print("LLVM IR preview:")
        for line in lines:
            print(f"  {line}")
        
    except Exception as e:
        print(f"Failed: {e}")
        import traceback
        traceback.print_exc()
        
        # Clean up
        if 'hugr_path' in locals():
            try:
                os.unlink(hugr_path)
            except:
                pass