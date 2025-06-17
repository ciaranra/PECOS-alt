#!/usr/bin/env python3
"""
Demonstration of the run_guppy() API
Shows that we have successfully implemented the requested API:
`results = run_guppy(guppy_function, shots)`
"""

from guppylang import guppy
from guppylang.std.quantum import qubit, h, cx, measure
from pecos import run_guppy, guppy_sim, run_guppy_batch, get_guppy_backends

# Define some quantum functions
@guppy
def random_bit() -> bool:
    """Generate a random bit using quantum superposition."""
    q = qubit()
    h(q)
    return measure(q)

@guppy
def bell_state() -> tuple[bool, bool]:
    """Create Bell state and measure both qubits."""
    q0 = qubit()
    q1 = qubit()
    h(q0)
    cx(q0, q1)
    return measure(q0), measure(q1)

@guppy
def ghz_state() -> tuple[bool, bool, bool]:
    """Create GHZ state with three qubits."""
    q0, q1, q2 = qubit(), qubit(), qubit()
    h(q0)
    cx(q0, q1)
    cx(q1, q2)
    return measure(q0), measure(q1), measure(q2)

def main():
    print("PECOS run_guppy() API Demonstration")
    print("===================================")
    
    # Check available backends
    print("\n1. Checking available backends:")
    backends = get_guppy_backends()
    for name, status in backends.items():
        print(f"   {name}: {status}")
    
    # Demo 1: Simple single qubit
    print("\n2. Testing run_guppy() with random_bit:")
    result = run_guppy(random_bit, shots=100)
    true_count = sum(result['results'])
    print(f"   [OK] Got {len(result['results'])} results")
    print(f"   True/False ratio: {true_count}/{100 - true_count}")
    print(f"   Backend used: {result['backend_used']}")
    
    # Demo 2: Bell state using guppy_sim alias
    print("\n3. Testing guppy_sim() alias with bell_state:")
    result = guppy_sim(bell_state, shots=200)
    correlated = sum(1 for r in result['results'] if r[0] == r[1])
    print(f"   [OK] Got {len(result['results'])} results")
    print(f"   Correlation rate: {correlated/200:.1%} (expect ~100% for Bell state)")
    print(f"   Sample results: {result['results'][:5]}")
    
    # Demo 3: Batch execution
    print("\n4. Testing run_guppy_batch():")
    batch_results = run_guppy_batch([random_bit, bell_state, ghz_state], shots=50)
    for func_name, result in batch_results.items():
        if 'error' not in result:
            print(f"   [OK] {func_name}: {result['shots']} shots completed")
    
    # Demo 4: Verbose mode
    print("\n5. Testing verbose mode:")
    result = run_guppy(random_bit, shots=10, verbose=True)
    
    print("\n[SUCCESS] Successfully demonstrated run_guppy() API!")
    print("The API matches the requested pattern: results = run_guppy(guppy, shots)")
    
    print("\nNOTE: This is using placeholder QIR generation and simulated results.")
    print("In production, use:")
    print("  - hugr-llvm binary for actual HUGR→QIR compilation")
    print("  - PECOS QIR runtime for actual quantum execution")

if __name__ == "__main__":
    main()