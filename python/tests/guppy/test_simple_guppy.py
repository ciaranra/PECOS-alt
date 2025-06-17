#\!/usr/bin/env python3
"""Test simple Guppy compilation"""

from guppylang import guppy
from guppylang.std.quantum import qubit, h, measure

@guppy
def random_bit() -> bool:
    """Generate a random bit using quantum superposition"""
    q = qubit()
    h(q)
    return measure(q)

if __name__ == "__main__":
    print("Testing Guppy compilation...")
    
    try:
        # Compile the function
        compiled = guppy.compile(random_bit)
        print("✅ Guppy function compiled successfully\!")
        
        # Show the HUGR
        print(f"\nCompiled function: {compiled}")
        print(f"Package: {compiled.package}")
        
        # Try to get HUGR bytes
        try:
            hugr_bytes = compiled.package.to_bytes()
            print(f"\n✅ HUGR bytes generated: {len(hugr_bytes)} bytes")
        except Exception as e:
            print(f"\n❌ Could not generate HUGR bytes: {e}")
            
    except Exception as e:
        print(f"❌ Compilation failed: {e}")
        import traceback
        traceback.print_exc()
EOF < /dev/null
