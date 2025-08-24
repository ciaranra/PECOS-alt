"""Test Guppy programs that properly output results for Selene to capture.

This shows how Guppy programs should use result() to tag final outputs
that Selene can extract from the result stream.
"""

from pathlib import Path
import tempfile

try:
    from guppylang import guppy
    from guppylang.std.quantum import qubit, h, cx, measure
    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False

# Check if Guppy has result function
try:
    from guppylang.std.io import result
    GUPPY_RESULT_AVAILABLE = True
except ImportError:
    GUPPY_RESULT_AVAILABLE = False
    # Try alternative import
    try:
        from guppylang.std import result
        GUPPY_RESULT_AVAILABLE = True
    except ImportError:
        pass


def test_guppy_with_explicit_results():
    """Create Guppy programs that properly output tagged results."""
    
    if not GUPPY_AVAILABLE:
        print("Guppy not available")
        return
    
    print("=" * 60)
    print("GUPPY PROGRAMS WITH RESULT TAGGING")
    print("=" * 60)
    
    # Check what I/O functions are available
    print("\nChecking Guppy I/O capabilities...")
    try:
        import guppylang.std.io as io
        print(f"guppylang.std.io available with: {dir(io)}")
    except ImportError:
        print("guppylang.std.io not found")
    
    try:
        import guppylang.std as std
        print(f"guppylang.std available with: {[x for x in dir(std) if not x.startswith('_')]}")
    except ImportError:
        print("guppylang.std not found")
    
    # Test 1: Simple measurement with result tagging
    if GUPPY_RESULT_AVAILABLE:
        @guppy
        def measure_with_result() -> None:
            """Measure a qubit and tag the result."""
            q = qubit()
            h(q)
            measurement = measure(q)
            # Tag the measurement with a name for Selene to capture
            result("measurement_outcome", measurement)
        
        print("\nProgram 1: measure_with_result defined")
    else:
        print("\nNote: result() function not available in this Guppy version")
        
        # Alternative: Return values become results
        @guppy
        def measure_with_return() -> bool:
            """Return measurement - this should appear in results."""
            q = qubit()
            h(q)
            return measure(q)
        
        print("\nProgram 1 (alternative): measure_with_return defined")
    
    # Test 2: Bell state with named results
    if GUPPY_RESULT_AVAILABLE:
        @guppy
        def bell_state_with_results() -> None:
            """Create Bell state and output named results."""
            q0, q1 = qubit(), qubit()
            h(q0)
            cx(q0, q1)
            
            # Measure and tag results
            m0 = measure(q0)
            m1 = measure(q1)
            
            result("qubit_0", m0)
            result("qubit_1", m1)
            # Could also output a combined result
            result("both_same", m0 == m1)  # Should always be True for Bell state
        
        print("Program 2: bell_state_with_results defined")
    else:
        @guppy
        def bell_state_with_return() -> tuple[bool, bool]:
            """Return Bell state measurements."""
            q0, q1 = qubit(), qubit()
            h(q0)
            cx(q0, q1)
            return measure(q0), measure(q1)
        
        print("Program 2 (alternative): bell_state_with_return defined")
    
    # Test 3: Multiple measurements with statistics
    if GUPPY_RESULT_AVAILABLE:
        @guppy
        def quantum_stats() -> None:
            """Perform multiple measurements and output statistics."""
            # Create 3 qubits in superposition
            q0, q1, q2 = qubit(), qubit(), qubit()
            h(q0)
            h(q1)
            h(q2)
            
            # Measure all
            m0 = measure(q0)
            m1 = measure(q1)
            m2 = measure(q2)
            
            # Output individual results
            result("bit_0", m0)
            result("bit_1", m1)
            result("bit_2", m2)
            
            # Output derived statistics
            count = int(m0) + int(m1) + int(m2)
            result("total_ones", count)
            result("all_same", (m0 == m1) and (m1 == m2))
        
        print("Program 3: quantum_stats defined")
    
    # Now compile these to HUGR and show structure
    print("\n" + "=" * 60)
    print("COMPILING TO HUGR")
    print("=" * 60)
    
    try:
        from pecos.compilation_pipeline import compile_guppy_to_hugr
        import json
        
        # Compile the appropriate program based on what's available
        if GUPPY_RESULT_AVAILABLE:
            if 'measure_with_result' in locals():
                hugr_bytes = compile_guppy_to_hugr(measure_with_result)
                prog_name = "measure_with_result"
            elif 'bell_state_with_results' in locals():
                hugr_bytes = compile_guppy_to_hugr(bell_state_with_results)
                prog_name = "bell_state_with_results"
            else:
                print("No programs with result() compiled")
                return
        else:
            hugr_bytes = compile_guppy_to_hugr(measure_with_return)
            prog_name = "measure_with_return"
        
        print(f"\nCompiled {prog_name} to HUGR: {len(hugr_bytes)} bytes")
        
        # Parse and examine HUGR structure
        hugr_json = json.loads(hugr_bytes.decode('utf-8'))
        
        # Look for result/output operations in the HUGR
        print("\nSearching HUGR for output operations...")
        
        def search_for_outputs(obj, path=""):
            """Recursively search for output-related operations."""
            if isinstance(obj, dict):
                # Check for output/result operations
                if 'op' in obj:
                    op = obj['op']
                    if any(term in str(op).lower() for term in ['output', 'result', 'return', 'io']):
                        print(f"  Found at {path}: {op}")
                
                # Recurse
                for key, value in obj.items():
                    search_for_outputs(value, f"{path}.{key}" if path else key)
            elif isinstance(obj, list):
                for i, item in enumerate(obj):
                    search_for_outputs(item, f"{path}[{i}]")
        
        search_for_outputs(hugr_json)
        
        # Save HUGR for inspection
        with tempfile.TemporaryDirectory() as tmpdir:
            hugr_file = Path(tmpdir) / f"{prog_name}.hugr"
            hugr_file.write_bytes(hugr_bytes)
            print(f"\nSaved HUGR to: {hugr_file}")
            
            # Also save a formatted version for readability
            hugr_pretty = Path(tmpdir) / f"{prog_name}_formatted.json"
            hugr_pretty.write_text(json.dumps(hugr_json, indent=2))
            print(f"Saved formatted HUGR to: {hugr_pretty}")
            
            # Show a snippet of the HUGR structure
            if 'modules' in hugr_json and hugr_json['modules']:
                module = hugr_json['modules'][0]
                if 'nodes' in module and module['nodes']:
                    print(f"\nHUGR has {len(module['nodes'])} nodes")
                    print("First few node types:")
                    for i, node in enumerate(module['nodes'][:5]):
                        if 'op' in node:
                            print(f"  Node {i}: {node['op']}")
        
    except ImportError:
        print("Compilation pipeline not available")
    except Exception as e:
        print(f"Compilation error: {e}")


def test_guppy_result_in_selene_context():
    """Show how Guppy results would be captured by Selene."""
    
    print("\n" + "=" * 60)
    print("HOW SELENE CAPTURES GUPPY RESULTS")  
    print("=" * 60)
    
    print("""
When a Guppy program uses result(tag, value), Selene captures it as:
1. During compilation: Guppy -> HUGR includes I/O operations
2. HUGR -> LLVM: Generates calls to __quantum__rt__result_record(tag, value)
3. During execution: Selene runtime intercepts these calls
4. Result stream: Tagged as ("USER:TYPE:tag", value)
5. Python receives: After parsing, just (tag, value)

Example flow:
    Guppy:  result("outcome", True)
    LLVM:   call void @__quantum__rt__result_record("outcome", i1 1)  
    Stream: ("USER:BOOL:outcome", True)
    Python: ("outcome", True)

For programs that return values instead:
    Guppy:  return (m0, m1)
    LLVM:   Returns struct/tuple
    Stream: ("result", (True, False))  # Default tagging
    Python: ("result", (True, False))
    """)


if __name__ == "__main__":
    test_guppy_with_explicit_results()
    test_guppy_result_in_selene_context()