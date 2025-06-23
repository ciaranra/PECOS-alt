#!/usr/bin/env python3
"""Simple test to verify QIR runtime works."""

import time
import sys
sys.path.append('python/pecos-rslib/rust')

def test_qir_reset():
    """Test if QIR runtime can be reset without hanging."""
    print("Testing QIR runtime reset...")
    
    try:
        # Import the Python bindings
        import pecos_rslib
        
        start_time = time.time()
        
        # Test simple reset
        pecos_rslib.reset_qir_runtime()
        
        reset_time = time.time() - start_time
        print(f"QIR runtime reset completed in {reset_time:.3f} seconds")
        
        if reset_time < 1.0:
            print("✓ QIR runtime reset is fast (< 1s)")
            return True
        else:
            print("✗ QIR runtime reset is slow (>= 1s)")
            return False
            
    except ImportError as e:
        print(f"✗ Could not import pecos_rslib: {e}")
        return False
    except Exception as e:
        print(f"✗ QIR runtime test failed: {e}")
        return False

if __name__ == "__main__":
    success = test_qir_reset()
    if success:
        print("\n✓ Simplified QIR runtime appears to be working correctly!")
    else:
        print("\n✗ There may be issues with the simplified QIR runtime.")