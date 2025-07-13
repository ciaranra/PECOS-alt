#!/usr/bin/env python3
"""Test different tuple sizes to find segfault threshold."""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import qubit, measure, x
from pecos.frontends import guppy_sim


def test_tuple_size(n: int):
    """Test returning n-tuple of bools."""
    print(f"\nTesting {n}-tuple of bools...")
    
    # Create the function dynamically
    func_str = f"""
@guppy
def test_func() -> tuple[{', '.join(['bool'] * n)}]:
    results = []
    for i in range({n}):
        q = qubit()
        if i % 2 == 0:
            x(q)
        r = measure(q)
        results.append(r)
    return ({', '.join([f'results[{i}]' for i in range(n)])})
"""
    
    # Execute the function definition
    namespace = {'guppy': guppy, 'qubit': qubit, 'measure': measure, 'x': x}
    exec(func_str, namespace)
    test_func = namespace['test_func']
    
    try:
        results = guppy_sim(test_func, max_qubits=10).run(5)
        print(f"  Success! Results: {results['_result'][:3]}...")
        return True
    except Exception as e:
        print(f"  Failed with error: {e}")
        return False


if __name__ == "__main__":
    print("Testing different tuple sizes...")
    
    # Test progressively larger tuples
    for size in [1, 2, 3, 4, 5, 6, 7, 8]:
        success = test_tuple_size(size)
        if not success:
            print(f"\nFailed at tuple size {size}")
            break
    else:
        print("\nAll sizes tested successfully!")