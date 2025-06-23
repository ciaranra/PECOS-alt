#!/usr/bin/env python3
"""Check execute_qir function signature"""

import sys
import os
import inspect
sys.path.append("python/quantum-pecos/src")

try:
    from pecos_rslib import execute_qir
    
    print("execute_qir signature:")
    sig = inspect.signature(execute_qir)
    print(f"  {sig}")
    
    # Try calling with positional args only
    result = execute_qir("test.ll", 1, 42, None, None)
    print(f"Positional-only call result: {result}")
    
except Exception as e:
    print(f"Error: {e}")
    import traceback
    traceback.print_exc()