#!/usr/bin/env python3
"""Debug simple test"""

import sys
import os
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.builtins import owned
from guppylang.std.quantum import qubit, measure
from pecos.frontends import guppy_sim

@guppy
def test_simple() -> bool:
    return True

@guppy
def test_simple_quantum() -> bool:
    q = qubit()
    return measure(q)

if __name__ == "__main__":
    print("Testing simple classical function...")
    try:
        results = guppy_sim(test_simple, max_qubits=10).run(1)
        print(f"Success: {results}")
    except Exception as e:
        print(f"Failed: {e}")
        import traceback
        traceback.print_exc()
    
    print("\nTesting simple quantum function...")
    try:
        results = guppy_sim(test_simple_quantum, max_qubits=10).run(1)
        print(f"Success: {results}")
    except Exception as e:
        print(f"Failed: {e}")
        import traceback
        traceback.print_exc()