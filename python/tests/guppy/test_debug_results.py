"""Debug test to see raw shot results."""
from guppylang import guppy
from guppylang.std.quantum import qubit, measure, x, y, z
from pecos.frontends import guppy_sim
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
    sim = guppy_sim(simple_test, max_qubits=4)
    results = sim.run(2)
    
    print("Raw results from guppy_sim:")
    print(f"Type: {type(results)}")
    print(f"Keys: {results.keys()}")
    print(f"Contents: {results}")
    
    # Check what we got
    if "_result" in results:
        print(f"\n_result type: {type(results['result'])}")
        print(f"_result length: {len(results['result'])}")
        if len(results['result']) > 0:
            print(f"First shot type: {type(results['result'][0])}")
            print(f"First shot value: {results['result'][0]}")
            
            # Decode if tuple
            if isinstance(results['result'][0], tuple):
                r1, r2, r3, r4 = results['result'][0]
                print(f"\nDecoded first shot:")
                print(f"  r1 (|0⟩) = {r1} (expected False)")
                print(f"  r2 (X|0⟩) = {r2} (expected True)")
                print(f"  r3 (Y|0⟩) = {r3} (expected True)")
                print(f"  r4 (Z|0⟩) = {r4} (expected False)")


if __name__ == "__main__":
    test_raw_results()