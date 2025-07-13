#!/usr/bin/env python3
"""Test that 4-tuple returns work without segfault."""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import qubit, measure
from pecos.frontends import guppy_sim


@guppy
def test_4tuple() -> tuple[bool, bool, bool, bool]:
    """Test returning 4-tuple of measurement results."""
    q1 = qubit()
    r1 = measure(q1)  # Should be False
    
    q2 = qubit()
    r2 = measure(q2)  # Should be False
    
    q3 = qubit()
    r3 = measure(q3)  # Should be False
    
    q4 = qubit()
    r4 = measure(q4)  # Should be False
    
    return r1, r2, r3, r4


if __name__ == "__main__":
    print("Testing 4-tuple returns...")
    
    try:
        # Run the simulation
        results = guppy_sim(test_4tuple, max_qubits=10).run(10)
        
        print(f"Success! Got results: {results}")
        
        # Check that all results are (False, False, False, False) (all qubits measured as |0⟩)
        for r in results["_result"]:
            assert r == (False, False, False, False), f"Expected (False, False, False, False), got {r}"
        
        print("✅ 4-tuple test passed!")
        
    except Exception as e:
        print(f"❌ Test failed: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)