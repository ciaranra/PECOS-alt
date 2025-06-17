#!/usr/bin/env python3
"""Minimal Guppy test"""

from guppylang.decorator import guppy
from guppylang.std.quantum import qubit, h, measure
from guppylang import guppy as guppy_compiler

@guppy
def random_bit() -> bool:
    """Generate random bit using superposition"""
    q = qubit()
    h(q)
    return measure(q)

if __name__ == "__main__":
    print("Compiling Guppy function...")
    try:
        # Compile the function directly
        compiled = guppy_compiler.compile(random_bit)
        print(f"✅ Function compiled: {type(compiled)}")
        
        # Get HUGR bytes
        hugr_bytes = compiled.package.to_bytes()
        print(f"✅ HUGR bytes: {len(hugr_bytes)} bytes")
        
        # Now test with GuppyFrontend
        import sys
        sys.path.insert(0, 'python/quantum-pecos/src')
        from pecos.frontends.guppy_frontend import GuppyFrontend
        
        frontend = GuppyFrontend()
        print(f"\n✅ GuppyFrontend created with backend: {frontend.get_backend_info()['backend']}")
        
        # Try compiling the function
        qir_file = frontend.compile_function(random_bit)
        print(f"✅ Compiled to QIR: {qir_file}")
        
        # Read and show QIR content
        with open(qir_file, 'r') as f:
            qir_content = f.read()
            print(f"\nQIR Preview ({len(qir_content)} chars):")
            print(qir_content[:500] + "..." if len(qir_content) > 500 else qir_content)
            
        frontend.cleanup()
        
    except Exception as e:
        print(f"❌ Error: {e}")
        import traceback
        traceback.print_exc()