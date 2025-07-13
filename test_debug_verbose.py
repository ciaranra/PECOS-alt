#!/usr/bin/env python3
"""Debug with verbose logging to see what's happening"""

import sys
import os
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import qubit, measure, x
from pecos.frontends import guppy_sim

@guppy
def test_conditional_verbose() -> bool:
    """Test conditional with verbose output"""
    q1 = qubit()
    x(q1)  # q1 = |1⟩ always
    
    q2 = qubit()  # q2 = |0⟩
    
    # This condition should always be True
    if measure(q1):  
        x(q2)  # Should always execute
    
    return measure(q2)  # Should always be True if conditional works

if __name__ == "__main__":
    # Enable verbose logging
    os.environ['LLVM_RUNTIME_QUIET'] = '0'  # Enable verbose output
    
    print("Testing conditional with verbose logging...")
    try:
        # Run with only 1 shot to see detailed logs
        results = guppy_sim(test_conditional_verbose, max_qubits=10).run(1)
        
        print(f"Results: {results}")
        
        # Extract result
        if isinstance(results['_result'][0], tuple):
            bool_result = results['_result'][0][-1]
        else:
            bool_result = results['_result'][0]
            
        print(f"Final result: {bool_result}")
        print(f"Expected: True (1)")
        print(f"Success: {bool_result == 1 or bool_result == True}")
        
    except Exception as e:
        print(f"Failed: {e}")
        import traceback
        traceback.print_exc()