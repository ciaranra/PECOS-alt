#!/usr/bin/env python3
"""Test if LLVM runtime is executing conditionals correctly"""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import qubit, measure, x
from pecos.frontends import guppy_sim
import tempfile

@guppy
def test_simple_true() -> bool:
    """Test that should always return True - control test"""
    q = qubit()
    x(q)  # Set to |1⟩
    return measure(q)  # Should always be True

@guppy
def test_simple_false() -> bool:
    """Test that should always return False - control test"""
    q = qubit()
    # q starts in |0⟩
    return measure(q)  # Should always be False

@guppy
def test_conditional_always_true() -> bool:
    """Test conditional where condition is always True"""
    q1 = qubit()
    x(q1)  # q1 = |1⟩ always
    
    q2 = qubit()  # q2 = |0⟩
    
    # This condition should always be True
    if measure(q1):  
        x(q2)  # Should always execute
    
    return measure(q2)  # Should always be True if conditional works

@guppy
def test_conditional_always_false() -> bool:
    """Test conditional where condition is always False"""
    q1 = qubit()
    # q1 = |0⟩ always
    
    q2 = qubit()  # q2 = |0⟩
    
    # This condition should always be False
    if measure(q1):  
        x(q2)  # Should never execute
    
    return measure(q2)  # Should always be False if conditional works

if __name__ == "__main__":
    tests = [
        ("Simple True (control)", test_simple_true, lambda r: all(r)),
        ("Simple False (control)", test_simple_false, lambda r: all(not x for x in r)),
        ("Conditional Always True", test_conditional_always_true, lambda r: all(r)),
        ("Conditional Always False", test_conditional_always_false, lambda r: all(not x for x in r)),
    ]
    
    for test_name, test_func, check_func in tests:
        print(f"\nTesting: {test_name}")
        try:
            results = guppy_sim(test_func, max_qubits=10).run(10)
            
            # Handle tuple results if present
            if isinstance(results['_result'][0], tuple):
                bool_results = [r[-1] for r in results['_result']]
            else:
                bool_results = results['_result']
            
            print(f"  Raw results: {results['_result'][:5]}...")  # Show first 5
            print(f"  Bool results: {bool_results}")
            passed = check_func(bool_results)
            print(f"  Result: {'PASS' if passed else 'FAIL'}")
            
            if not passed:
                print(f"  ERROR: Expected condition not met!")
                
        except Exception as e:
            print(f"  EXCEPTION: {e}")
            import traceback
            traceback.print_exc()