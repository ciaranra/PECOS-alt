#!/usr/bin/env python3
"""Test edge cases for measurement conversion."""

from guppylang import guppy
from guppylang.std.quantum import h, measure, qubit, x, cx
import tempfile
import sys
import os

# Add the PECOS Python package to the path
sys.path.insert(0, '/home/ciaranra/Repos/cl_projects/gup/PECOS/python/quantum-pecos/src')
sys.path.insert(0, '/home/ciaranra/Repos/cl_projects/gup/PECOS/python/pecos-rslib/src')

@guppy
def single_x_gate() -> bool:
    """X gate test - should return 1 always."""
    q = qubit()
    x(q)
    return measure(q)

@guppy
def bell_state() -> tuple[bool, bool]:
    """Bell state test - should return correlated measurements."""
    q1 = qubit()
    q2 = qubit()
    h(q1)
    cx(q1, q2)
    return measure(q1), measure(q2)

@guppy
def multiple_measurements() -> tuple[bool, bool, bool]:
    """Multiple independent measurements."""
    q1 = qubit()
    q2 = qubit()
    q3 = qubit()
    h(q1)
    h(q2) 
    x(q3)  # This should always be 1
    return measure(q1), measure(q2), measure(q3)

def test_edge_case(func, name, expected_behavior):
    """Test a specific edge case."""
    print(f"\n=== Testing {name} ===")
    
    # Compile the function
    compiled = guppy.compile_function(func)
    hugr_bytes = compiled.package.to_bytes()
    
    import pecos_rslib.hugr_qir as hugr_qir
    
    with tempfile.NamedTemporaryFile(mode='wb', suffix='.hugr', delete=False) as hugr_file:
        hugr_file.write(hugr_bytes)
        hugr_path = hugr_file.name
    
    try:
        # Compile with HUGR convention
        hugr_llvm = hugr_qir.compile_hugr_to_llvm_rust(hugr_path, llvm_convention="hugr")
        
        # Check conversion
        deferred_calls = hugr_llvm.count('__hugr__quantum__qis__m__body')
        result_getters = hugr_llvm.count('__quantum__rt__result_get_one')
        immediate_calls = hugr_llvm.count('call i32 @__quantum__qis__m__body(')
        undefined_vars = hugr_llvm.count('%is_one = icmp ne i32 %measurement_result, 0') - hugr_llvm.count('%measurement_result = call i32 @__quantum__rt__result_get_one')
        
        print(f"  Deferred calls: {deferred_calls}")
        print(f"  Result getters: {result_getters}")
        print(f"  Immediate calls: {immediate_calls}")
        print(f"  Undefined vars: {max(0, undefined_vars)}")
        
        conversion_ok = (deferred_calls > 0 and result_getters == deferred_calls and 
                        immediate_calls == 0 and undefined_vars <= 0)
        
        if conversion_ok:
            print("  ✓ Conversion successful")
            
            # Test execution
            try:
                engine = hugr_qir.create_qir_engine_from_hugr_rust(
                    hugr_bytes, 
                    shots=20,
                    llvm_convention="hugr"
                )
                
                results = engine.run()
                print(f"  Executed {len(results)} shots")
                print(f"  Results: {results}")
                
                # Check expected behavior
                if expected_behavior == "always_1":
                    if all(r == 1 for r in results):
                        print("  ✓ Results match expectation (always 1)")
                    else:
                        print("  ⚠ Results don't match expectation (should be always 1)")
                        
                elif expected_behavior == "always_0":
                    if all(r == 0 for r in results):
                        print("  ✓ Results match expectation (always 0)")
                    else:
                        print("  ⚠ Results don't match expectation (should be always 0)")
                        
                elif expected_behavior == "random":
                    ones = sum(results)
                    if 0.2 <= ones/len(results) <= 0.8:  # Allow for statistical variation
                        print("  ✓ Results show expected randomness")
                    else:
                        print("  ⚠ Results may not be random enough")
                        
                elif expected_behavior == "correlated":
                    # For Bell state, results should be [(0,0), (0,0), (1,1), (1,1), ...]
                    # Convert flat list to pairs
                    if len(results) % 2 == 0:
                        pairs = [(results[i], results[i+1]) for i in range(0, len(results), 2)]
                        correlated = all(p[0] == p[1] for p in pairs)
                        if correlated:
                            print("  ✓ Results show expected correlation")
                        else:
                            print("  ⚠ Results don't show expected correlation")
                            print(f"    Pairs: {pairs}")
                    else:
                        print("  ⚠ Odd number of results for pair test")
                        
                elif expected_behavior == "mixed":
                    # For multiple measurements, expect some randomness but X gate always 1
                    if len(results) % 3 == 0:
                        triples = [(results[i], results[i+1], results[i+2]) for i in range(0, len(results), 3)]
                        third_always_1 = all(t[2] == 1 for t in triples)
                        if third_always_1:
                            print("  ✓ Third measurement always 1 (X gate working)")
                        else:
                            print("  ⚠ Third measurement not always 1")
                            print(f"    Triples: {triples}")
                    else:
                        print("  ⚠ Results not in triples")
                        
            except Exception as e:
                print(f"  ✗ Execution failed: {e}")
        else:
            print("  ✗ Conversion failed")
            
    except Exception as e:
        print(f"  ✗ Compilation failed: {e}")
        
    finally:
        os.unlink(hugr_path)

def main():
    print("="*60)
    print("TESTING MEASUREMENT EDGE CASES")
    print("="*60)
    
    test_edge_case(single_x_gate, "X Gate Test", "always_1")
    test_edge_case(bell_state, "Bell State Test", "correlated") 
    test_edge_case(multiple_measurements, "Multiple Measurements", "mixed")
    
    print("\n" + "="*60)
    print("EDGE CASE TESTING COMPLETE")
    print("="*60)

if __name__ == "__main__":
    main()