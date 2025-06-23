#!/usr/bin/env python3
"""Test QIR runtime functions directly without full execution pipeline"""

import sys
import os
sys.path.append("python/quantum-pecos/src")

def test_qir_runtime_functions():
    """Test individual QIR runtime functions"""
    print("=== Testing QIR Runtime Functions Directly ===")
    
    try:
        from pecos_rslib import reset_qir_runtime
        
        print("Step 1: Reset QIR runtime")
        reset_qir_runtime()
        print("✅ Reset successful")
        
        # Test if we can call QIR runtime functions directly
        # This bypasses the whole LLVM JIT compilation pipeline
        print("Step 2: Available functions in pecos_rslib:")
        import pecos_rslib
        funcs = [attr for attr in dir(pecos_rslib) if not attr.startswith('_') and callable(getattr(pecos_rslib, attr, None))]
        for func in funcs:
            print(f"  - {func}")
        
        # Check if there are any direct QIR execution functions that bypass the complex pipeline
        print("\nStep 3: Looking for simpler execution methods...")
        
        # Try to find functions that might run QIR without the full compilation pipeline
        possible_funcs = [func for func in funcs if 'qir' in func.lower() and func != 'reset_qir_runtime']
        print("QIR-related functions:", possible_funcs)
        
        return True
        
    except Exception as e:
        print(f"❌ Failed: {e}")
        import traceback
        traceback.print_exc()
        return False

def test_simple_qir_without_jit():
    """See if we can run QIR operations without JIT compilation"""
    print("\n=== Testing if we can avoid LLVM JIT entirely ===")
    
    # The idea: maybe we can call the quantum runtime functions directly
    # without going through the LLVM compilation pipeline
    
    try:
        from pecos_rslib import reset_qir_runtime
        
        reset_qir_runtime()
        
        print("Can we access quantum engines directly?")
        import pecos_rslib
        
        # Look for quantum engines that might work without QIR
        engines = [attr for attr in dir(pecos_rslib) if 'engine' in attr.lower()]
        print("Available engines:", engines)
        
        # Look for direct simulation functions
        sim_funcs = [attr for attr in dir(pecos_rslib) if 'sim' in attr.lower()]
        print("Simulation functions:", sim_funcs)
        
        # Maybe we can use the quantum engines directly?
        if 'StateVecEngineRs' in engines:
            print("\nTrying StateVecEngineRs directly...")
            engine_class = getattr(pecos_rslib, 'StateVecEngineRs')
            engine = engine_class()
            print(f"Created engine: {engine}")
            print(f"Engine methods: {[m for m in dir(engine) if not m.startswith('_')]}")
        
        return True
        
    except Exception as e:
        print(f"❌ Failed: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    test_qir_runtime_functions()
    test_simple_qir_without_jit()
    
    print(f"\n=== Analysis ===")
    print("The hang/segfault seems to occur in the LLVM JIT compilation phase.")
    print("If we can identify a simpler execution path that bypasses LLVM JIT,")
    print("we might be able to isolate whether the issue is:")
    print("1. LLVM JIT specific (compilation/loading)")
    print("2. Quantum runtime specific (function calls)")
    print("3. PECOS engine specific (simulation)")