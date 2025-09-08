"""Debug test to see raw shot results."""
from guppylang import guppy
from guppylang.std.quantum import qubit, measure, x, y, z
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



def test_raw_results():
    """Test to see raw shot data."""
    @guppy
    def simple_test() -> tuple[bool, bool, bool, bool]:
        q1 = qubit()  # |0⟩ -> False
        r1 = measure(q1)
        
        q2 = qubit()
        x(q2)  # |1⟩ -> True
        r2 = measure(q2)
        
        q3 = qubit()
        y(q3)  # Y|0⟩ = i|1⟩ -> True
        r3 = measure(q3)
        
        q4 = qubit()
        z(q4)  # Z|0⟩ = |0⟩ -> False
        r4 = measure(q4)
        
        return r1, r2, r3, r4
    
    # Run simulation
    sim_instance = sim(simple_test).qubits(4).quantum(state_vector())
    results = sim_instance.run(2)
    
    print("Raw results from sim:")
    print(f"Type: {type(results)}")
    print(f"Keys: {results.keys()}")
    print(f"Contents: {results}")
    
    # Check what we got
    if "_result" in results:
        print(f"\n_result type: {type(results.get("measurements", results.get("measurement_1", [])))}")
        print(f"_result length: {len(results.get("measurements", results.get("measurement_1", [])))}")
        if len(results.get("measurements", results.get("measurement_1", []))) > 0:
            print(f"First shot type: {type(results.get("measurements", results.get("measurement_1", []))[0])}")
            print(f"First shot value: {results.get("measurements", results.get("measurement_1", []))[0]}")
            
            # Decode if tuple
            if isinstance(results.get("measurements", results.get("measurement_1", []))[0], tuple):
                r1, r2, r3, r4 = results.get("measurements", results.get("measurement_1", []))[0]
                print(f"\nDecoded first shot:")
                print(f"  r1 (|0⟩) = {r1} (expected False)")
                print(f"  r2 (X|0⟩) = {r2} (expected True)")
                print(f"  r3 (Y|0⟩) = {r3} (expected True)")
                print(f"  r4 (Z|0⟩) = {r4} (expected False)")


if __name__ == "__main__":
    test_raw_results()