#!/usr/bin/env python3
"""Debug conditional operations - simplified"""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import qubit, measure, x
from pecos.frontends import guppy_sim

@guppy
def test_basic_conditional() -> bool:
    """Test: q1=|1⟩, if measure(q1) then x(q2), return measure(q2)"""
    q1 = qubit()
    x(q1)  # q1 = |1⟩
    
    q2 = qubit()  # q2 = |0⟩
    
    # The key test: does conditional execution work?
    measurement_result = measure(q1)  # Should be True
    if measurement_result:
        x(q2)  # Should flip q2 to |1⟩
    
    return measure(q2)  # Should be True if conditional worked

@guppy
def test_no_conditional() -> bool:
    """Control test: same logic but without conditional"""
    q1 = qubit()
    x(q1)  # q1 = |1⟩
    
    q2 = qubit()  # q2 = |0⟩
    
    # Always apply X (no conditional)
    measure(q1)  # Consume the measurement
    x(q2)  # Always flip q2
    
    return measure(q2)  # Should always be True

if __name__ == "__main__":
    print("Testing basic conditional (if it works, should be all True)...")
    try:
        results = guppy_sim(test_basic_conditional, max_qubits=10).run(10)
        print(f"Results: {results['_result']}")
        
        # Extract just the boolean values (ignore tuple structure if present)
        if isinstance(results['_result'][0], tuple):
            bool_results = [r[-1] for r in results['_result']]  # Get last element
        else:
            bool_results = results['_result']
            
        print(f"Boolean results: {bool_results}")
        success_rate = sum(bool_results) / len(bool_results)
        print(f"Success rate: {success_rate}")
        
    except Exception as e:
        print(f"Failed: {e}")
        import traceback
        traceback.print_exc()
    
    print("\nTesting control (no conditional, should be all True)...")
    try:
        results = guppy_sim(test_no_conditional, max_qubits=10).run(10)
        print(f"Results: {results['_result']}")
        
        # Extract just the boolean values
        if isinstance(results['_result'][0], tuple):
            bool_results = [r[-1] for r in results['_result']]
        else:
            bool_results = results['_result']
            
        print(f"Boolean results: {bool_results}")
        success_rate = sum(bool_results) / len(bool_results)
        print(f"Success rate: {success_rate}")
        
    except Exception as e:
        print(f"Failed: {e}")
        import traceback
        traceback.print_exc()