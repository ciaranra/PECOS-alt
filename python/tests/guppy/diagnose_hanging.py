#!/usr/bin/env python3
"""Diagnose where the hanging occurs in the Guppy pipeline."""

import sys
import time
sys.path.append('python/quantum-pecos/src')

try:
    from guppylang import guppy
    from guppylang.std.quantum import h, measure, qubit
    from pecos.frontends.run_guppy import run_guppy
except ImportError as e:
    print(f"Import error: {e}")
    sys.exit(1)

print("Diagnosing Guppy pipeline performance...")
print("="*60)

@guppy
def simple_test() -> bool:
    q = qubit()
    h(q)
    return measure(q)

# Test 1: Compilation only
print("\n1. Testing compilation speed...")
start = time.time()
try:
    from guppylang.decorator import guppy as guppy_decorator
    compiled = guppy_decorator.compile_function(simple_test)
    hugr_bytes = compiled.package.to_bytes()
    compile_time = time.time() - start
    print(f"   ✓ Guppy→HUGR compilation: {compile_time:.3f}s")
except Exception as e:
    print(f"   ✗ Compilation failed: {e}")

# Test 2: Frontend creation
print("\n2. Testing frontend creation...")
start = time.time()
try:
    from pecos.frontends.guppy_frontend import GuppyFrontend
    frontend = GuppyFrontend(use_rust_backend=True)
    frontend_time = time.time() - start
    print(f"   ✓ Frontend creation: {frontend_time:.3f}s")
except Exception as e:
    print(f"   ✗ Frontend creation failed: {e}")

# Test 3: QIR execution setup
print("\n3. Testing QIR runtime...")
start = time.time()
try:
    from pecos_rslib import reset_qir_runtime
    reset_qir_runtime()
    reset_time = time.time() - start
    print(f"   ✓ QIR runtime reset: {reset_time:.3f}s")
except Exception as e:
    print(f"   ✗ QIR runtime reset failed: {e}")

# Test 4: Single shot execution
print("\n4. Testing single shot execution...")
start = time.time()
try:
    result = run_guppy(simple_test, shots=1, verbose=False)
    single_shot_time = time.time() - start
    print(f"   ✓ Single shot execution: {single_shot_time:.3f}s")
    print(f"   Result: {result['results']}")
except Exception as e:
    print(f"   ✗ Single shot failed: {e}")

# Test 5: Multiple shots (small number)
print("\n5. Testing 5 shots...")
start = time.time()
try:
    result = run_guppy(simple_test, shots=5, verbose=False)
    multi_shot_time = time.time() - start
    print(f"   ✓ 5 shots execution: {multi_shot_time:.3f}s")
    print(f"   Results: {result['results']}")
    print(f"   Time per shot: {multi_shot_time/5:.3f}s")
except Exception as e:
    print(f"   ✗ 5 shots failed: {e}")

# Test 6: Check for memory/resource leaks
print("\n6. Testing repeated executions...")
times = []
for i in range(3):
    start = time.time()
    try:
        result = run_guppy(simple_test, shots=1, verbose=False)
        exec_time = time.time() - start
        times.append(exec_time)
        print(f"   Run {i+1}: {exec_time:.3f}s")
    except Exception as e:
        print(f"   Run {i+1} failed: {e}")
        break

if times:
    avg_time = sum(times) / len(times)
    print(f"   Average time: {avg_time:.3f}s")
    if times[-1] > times[0] * 2:
        print("   ⚠️  WARNING: Execution time increasing - possible resource leak")

print("\n" + "="*60)
print("Diagnosis complete!")
print("\nIf execution is slow (>3s per shot), the issue is likely:")
print("- QIR runtime initialization overhead")
print("- Quantum simulation overhead")  
print("- Resource cleanup issues")
print("\nRecommendations:")
print("- Use fewer shots in tests (1-5 max)")
print("- Use verbose=False in tests")
print("- Consider mocking run_guppy for unit tests")