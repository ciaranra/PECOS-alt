#!/usr/bin/env python3
"""Minimal QIR test - as close to pure Rust as possible"""

import sys
sys.path.append("python/quantum-pecos/src")

def test_minimal():
    print("=== Minimal QIR Test ===")
    
    # Check what's available
    import pecos_rslib
    print("Available in pecos_rslib:", [x for x in dir(pecos_rslib) if not x.startswith('_')])
    
    # Try to use QIR engine directly, bypassing all the wrapper complexity
    try:
        # Import from the internal module
        from pecos_rslib._pecos_rslib import QirEngine
        print("✅ Found QirEngine class")
        
        # Create engine just like Rust does
        engine = QirEngine("examples/qir/bell.ll")
        print("✅ Created QirEngine")
        
        # Try to execute - note: execute takes 4 params after self
        result = engine.execute(5, 42, None, None)
        print(f"✅ Execution result: {result}")
        
    except ImportError as e:
        print(f"❌ Import error: {e}")
        
    except Exception as e:
        print(f"❌ Error: {type(e).__name__}: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    test_minimal()