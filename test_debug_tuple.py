#!/usr/bin/env python3
"""Debug tuple test to reproduce TermSer error"""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import qubit, measure
from pecos.frontends import guppy_sim

@guppy
def test_3tuple() -> tuple[bool, bool, bool]:
    q1 = qubit()
    r1 = measure(q1)
    
    q2 = qubit()
    r2 = measure(q2)
    
    q3 = qubit()
    r3 = measure(q3)
    
    return r1, r2, r3

@guppy
def test_4tuple() -> tuple[bool, bool, bool, bool]:
    q1 = qubit()
    r1 = measure(q1)
    
    q2 = qubit()
    r2 = measure(q2)
    
    q3 = qubit()
    r3 = measure(q3)
    
    q4 = qubit()
    r4 = measure(q4)
    
    return r1, r2, r3, r4

if __name__ == "__main__":
    print("Testing 3-tuple...")
    try:
        results = guppy_sim(test_3tuple, max_qubits=10).run(2)
        print(f"Success: {results}")
    except Exception as e:
        print(f"Failed: {e}")
        print("This might be the TermSer error!")
        import traceback
        traceback.print_exc()
    
    print("\nTesting 4-tuple...")
    try:
        results = guppy_sim(test_4tuple, max_qubits=10).run(2)
        print(f"Success: {results}")
    except Exception as e:
        print(f"Failed: {e}")
        print("This might be the TermSer error!")
        import traceback
        traceback.print_exc()