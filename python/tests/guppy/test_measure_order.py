"""Test measurement ordering."""
from guppylang import guppy
from guppylang.std.quantum import qubit, measure, x
from pecos.frontends.guppy_api import sim
from pecos_rslib import state_vector
from typing import List, Tuple


def decode_integer_results(results: List[int], n_bits: int) -> List[Tuple[bool, ...]]:
    """Decode integer-encoded results back to tuples of booleans."""
    decoded = []
    for val in results:
        bits = []
        for i in range(n_bits):
            bits.append(bool(val & (1 << i)))
        decoded.append(tuple(bits))
    return decoded



def test_measure_order():
    """Test if measurements are returned in the right order."""
    @guppy
    def measure_order_test() -> tuple[bool, bool, bool, bool]:
        # Create 4 qubits with different states
        q1 = qubit()  # |0⟩ -> False
        
        q2 = qubit()
        x(q2)  # |1⟩ -> True
        
        q3 = qubit()  # |0⟩ -> False
        
        q4 = qubit()
        x(q4)  # |1⟩ -> True
        
        # Measure in order
        r1 = measure(q1)
        r2 = measure(q2)
        r3 = measure(q3)
        r4 = measure(q4)
        
        return r1, r2, r3, r4
    
    results = sim(measure_order_test).qubits(4).quantum(state_vector()).run(5)
    
    # Decode integer-encoded results
    decoded_results = decode_integer_results(results.get("measurements", results.get("measurement_1", [])), 4)
    for i, val in enumerate(decoded_results):
        r1, r2, r3, r4 = val
        print(f"Shot {i}: ({r1}, {r2}, {r3}, {r4})")
        print(f"  Expected: (False, True, False, True)")
        print(f"  Got:      ({r1}, {r2}, {r3}, {r4})")
        
        # Check pattern
        if r1 == r2 == r3 == r4:
            print("  ERROR: All values are the same!")
        elif r1 == False and r2 == True and r3 == False and r4 == True:
            print("  ✓ Correct order!")
        else:
            print("  ERROR: Wrong pattern!")
            

if __name__ == "__main__":
    test_measure_order()