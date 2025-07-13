#!/usr/bin/env python3
"""Debug conditional operations"""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import qubit, measure, x
from pecos.frontends import guppy_sim

@guppy
def test_simple_conditional() -> bool:
    """Simple conditional: if measure(q1): x(q2)"""
    q1 = qubit()
    x(q1)  # Put q1 in |1⟩, so measure should return True
    
    q2 = qubit()  # q2 starts in |0⟩
    
    if measure(q1):  # This should be True
        x(q2)  # So q2 should be flipped to |1⟩
    
    return measure(q2)  # Should return True

@guppy 
def test_simple_conditional_false() -> bool:
    """Simple conditional with False condition"""
    q1 = qubit()
    # q1 starts in |0⟩, so measure should return False
    
    q2 = qubit()  # q2 starts in |0⟩
    
    if measure(q1):  # This should be False
        x(q2)  # So this should not execute
    
    return measure(q2)  # Should return False

if __name__ == "__main__":
    print("Testing simple conditional (should always return True)...")
    try:
        results = guppy_sim(test_simple_conditional, max_qubits=10).run(10)
        print(f"Results: {results['_result']}")
        all_true = all(r for r in results['_result'])
        print(f"All True: {all_true}")
        if not all_true:
            print("ERROR: Conditional operations not working properly!")
    except Exception as e:
        print(f"Failed: {e}")
        import traceback
        traceback.print_exc()
    
    print("\nTesting simple conditional (should always return False)...")
    try:
        results = guppy_sim(test_simple_conditional_false, max_qubits=10).run(10)
        print(f"Results: {results['_result']}")
        all_false = all(not r for r in results['_result'])
        print(f"All False: {all_false}")
        if not all_false:
            print("ERROR: Conditional operations not working properly!")
    except Exception as e:
        print(f"Failed: {e}")
        import traceback
        traceback.print_exc()