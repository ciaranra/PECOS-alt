#!/usr/bin/env python3
"""Debug boolean expressions test"""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import qubit, measure
from pecos.frontends import guppy_sim

@guppy
def test_boolean_operations() -> bool:
    """Test basic boolean operations."""
    # Test boolean literals and operations
    a = True
    b = False
    c = a and b    # False
    d = a or b     # True  
    e = not a      # False
    f = not b      # True
    
    # Result should be: False or True or False or True = True
    return c or d or e or f

if __name__ == "__main__":
    print("Testing boolean expressions...")
    try:
        results = guppy_sim(test_boolean_operations, max_qubits=10).run(5)
        print(f"Success: {results}")
        # Should be all True
        for result in results['_result']:
            if result != 1:  # 1 = True
                print(f"ERROR: Expected 1, got {result}")
    except Exception as e:
        print(f"Failed: {e}")
        import traceback
        traceback.print_exc()