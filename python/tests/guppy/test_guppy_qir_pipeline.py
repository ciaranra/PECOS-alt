#!/usr/bin/env python3
"""
Test the complete Guppy → HUGR → Standard QIR → PECOS pipeline

This tests the new Standard QIR+ architecture implementation.
"""

import sys
sys.path.append('python/quantum-pecos/src')

from pecos.frontends.run_guppy import run_guppy, get_guppy_backends
from pecos.frontends.guppy_frontend import GuppyFrontend

def test_backend_availability():
    """Test that backends are properly detected"""
    print("=== Testing Backend Availability ===")
    backends = get_guppy_backends()
    print(f"Available backends: {backends}")
    
    if backends['guppy_available']:
        print("[PASS] Guppy is available")
    else:
        print("[FAIL] Guppy is not available - install with: pip install guppylang")
    
    if backends['rust_backend']:
        print("[PASS] Rust HUGR backend is available")
    else:
        print(f"[FAIL] Rust HUGR backend is not available: {backends.get('rust_message', 'Unknown')}")
    
    print(f"[OK] External tools available: {backends['external_tools']}")
    print()

def test_guppy_frontend():
    """Test the GuppyFrontend class directly"""
    print("=== Testing GuppyFrontend ===")
    
    try:
        frontend = GuppyFrontend(naming_convention="standard")
        info = frontend.get_backend_info()
        print(f"Frontend backend info: {info}")
        print(f"[OK] Using backend: {info['backend']}")
        print(f"[OK] Naming convention: {info['naming_convention']}")
        print()
        return True
    except Exception as e:
        print(f"[FAIL] Failed to create GuppyFrontend: {e}")
        return False

def test_simple_guppy_function():
    """Test with a simple Guppy function (if available)"""
    print("=== Testing Simple Guppy Function ===")
    
    try:
        from guppylang import guppy
        from guppylang.std.quantum import qubit, h, measure
        
        @guppy
        def random_bit() -> bool:
            """Generate a random bit using quantum superposition"""
            q = qubit()
            h(q)
            return measure(q)
        
        print("[PASS] Guppy function defined successfully")
        
        # Test compilation only (not execution)
        try:
            frontend = GuppyFrontend()
            qir_file = frontend.compile_function(random_bit)
            print(f"[PASS] Compiled to QIR: {qir_file}")
            
            # Read and display part of the QIR
            with open(qir_file, 'r') as f:
                qir_content = f.read()
                print("\nGenerated QIR (first 500 chars):")
                print(qir_content[:500])
                print("...")
                
        except Exception as e:
            print(f"[FAIL] Compilation failed: {e}")
            
    except ImportError:
        print("[SKIP] Guppy not available - skipping function test")
        print("  Install with: pip install guppylang")
    except Exception as e:
        print(f"[FAIL] Test failed: {e}")

def test_bell_state_function():
    """Test with a Bell state function (if Guppy available)"""
    print("\n=== Testing Bell State Function ===")
    
    try:
        from guppylang import guppy
        from guppylang.std.quantum import qubit, h, cx, measure
        
        @guppy
        def bell_state() -> tuple[bool, bool]:
            """Create a Bell state and measure both qubits"""
            q0, q1 = qubit(), qubit()
            h(q0)
            cx(q0, q1)
            return measure(q0), measure(q1)
        
        print("[PASS] Bell state function defined")
        
        try:
            # Test using run_guppy API
            result = run_guppy(bell_state, shots=10, verbose=True)
            print(f"\n[PASS] Execution completed!")
            print(f"  Function: {result['function_name']}")
            print(f"  Backend used: {result['backend_used']}")
            print(f"  Results (first 10): {result['results'][:10]}")
            print(f"  QIR file: {result['qir_file']}")
            
            # Check correlation
            if result['results']:
                correlated = sum(1 for (a, b) in result['results'] if a == b)
                print(f"  Correlation: {correlated}/{len(result['results'])} = {correlated/len(result['results']):.2%}")
                
        except Exception as e:
            print(f"[FAIL] Execution failed: {e}")
            import traceback
            traceback.print_exc()
            
    except ImportError:
        print("[SKIP] Guppy not available - skipping Bell state test")
    except Exception as e:
        print(f"[FAIL] Test failed: {e}")

def test_rust_compilation():
    """Test Rust compilation status"""
    print("\n=== Testing Rust Compilation ===")
    
    import subprocess
    import os
    
    try:
        # Check if pecos-qir compiled with hugr-support
        result = subprocess.run(
            ["cargo", "check", "-p", "pecos-qir", "--features", "hugr-support"],
            capture_output=True,
            text=True,
            cwd=os.path.dirname(os.path.abspath(__file__))
        )
        
        if result.returncode == 0:
            print("[PASS] pecos-qir compiles with hugr-support feature")
        else:
            print("[FAIL] pecos-qir compilation failed:")
            print(result.stderr[:500])
            
    except Exception as e:
        print(f"[FAIL] Could not check Rust compilation: {e}")

def main():
    """Run all tests"""
    print("Testing Guppy → HUGR → Standard QIR → PECOS Pipeline")
    print("=" * 60)
    
    test_backend_availability()
    
    if test_guppy_frontend():
        test_simple_guppy_function()
        test_bell_state_function()
    
    test_rust_compilation()
    
    print("\n" + "=" * 60)
    print("Testing complete!")

if __name__ == "__main__":
    main()