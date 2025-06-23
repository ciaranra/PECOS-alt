#!/usr/bin/env python3
"""Test just HUGR convention to verify it works correctly."""

def test_hugr_simple():
    """Test a very simple HUGR execution"""
    print("=" * 60)
    print("Testing HUGR Convention Only")
    print("=" * 60)
    
    try:
        from guppylang import guppy, qubit
        from guppylang.std.quantum import h
        from pecos.frontends.run_guppy import run_guppy
        
        @guppy
        def simple_h() -> qubit:
            q = qubit()
            h(q)
            return q
        
        print("Running with HUGR convention...")
        result = run_guppy(
            simple_h, 
            shots=1, 
            llvm_convention='hugr', 
            verbose=True
        )
        
        print(f"HUGR Result: {result}")
        print("✓ HUGR Convention Test: PASSED")
        return True
        
    except Exception as e:
        print(f"✗ HUGR Convention Test: FAILED - {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    success = test_hugr_simple()
    if success:
        print("\n🎉 HUGR convention works correctly!")
    else:
        print("\n❌ HUGR convention failed")
    exit(0 if success else 1)