#!/usr/bin/env python3
"""Test HUGR measurement handling"""

import os
import sys
sys.path.append('python/quantum-pecos/src')
from pecos_rslib import RustHugrCompiler, compile_hugr_to_qir_rust, RustHugrQirEngine

def test_simple_measurement():
    """Test a simple HUGR circuit with measurement"""
    
    # Create a minimal HUGR with magic number
    # HUGR magic number: 0x4855475269484A76
    hugr_bytes = b'HUGRiHJv' + b'''
{
  "version": "v1",
  "nodes": [{
    "parent": 0,
    "op": "Module"
  }, {
    "parent": 0,
    "op": "FuncDefn",
    "name": "_hugr_simple_quantum",
    "signature": {
      "params": [],
      "body": {
        "input": [],
        "output": [{
          "t": "Opaque",
          "extension": "prelude",
          "id": "bool",
          "args": [],
          "bound": "C"
        }],
        "extension_reqs": ["prelude", "tket2.quantum"]
      }
    }
  }, {
    "parent": 1,
    "op": "Input",
    "types": []
  }, {
    "parent": 1,
    "op": "Output",
    "types": [{
      "t": "Opaque",
      "extension": "prelude",
      "id": "bool",
      "args": [],
      "bound": "C"
    }]
  }, {
    "parent": 1,
    "op": "DFG",
    "signature": {
      "input": [],
      "output": [{
        "t": "Opaque",
        "extension": "prelude",
        "id": "bool",
        "args": [],
        "bound": "C"
      }],
      "extension_reqs": ["prelude", "tket2.quantum"]
    }
  }, {
    "parent": 4,
    "op": "Input",
    "types": []
  }, {
    "parent": 4,
    "op": "Output",
    "types": [{
      "t": "Opaque",
      "extension": "prelude",
      "id": "bool",
      "args": [],
      "bound": "C"
    }]
  }, {
    "parent": 4,
    "op": {
      "op_name": "QAlloc",
      "extension": "tket2.quantum",
      "args": [],
      "signature": {
        "input": [],
        "output": [{
          "t": "Q",
          "b": "B"
        }],
        "extension_reqs": []
      }
    }
  }, {
    "parent": 4,
    "op": {
      "op_name": "H",
      "extension": "tket2.quantum",
      "args": [],
      "signature": {
        "input": [{
          "t": "Q",
          "b": "B"
        }],
        "output": [{
          "t": "Q",
          "b": "B"
        }],
        "extension_reqs": []
      }
    }
  }, {
    "parent": 4,
    "op": {
      "op_name": "MeasureFree",
      "extension": "tket2.quantum",
      "args": [],
      "signature": {
        "input": [{
          "t": "Q",
          "b": "B"
        }],
        "output": [{
          "t": "Opaque",
          "extension": "prelude",
          "id": "bool",
          "args": [],
          "bound": "C"
        }],
        "extension_reqs": []
      }
    }
  }],
  "edges": [
    [5, 0, 6, 0],
    [7, 0, 8, 0],
    [8, 0, 9, 0],
    [9, 0, 6, 0],
    [6, 0, 3, 0],
    [1, 0, 4],
    [4, 0, 1]
  ],
  "metadata": [{
    "name": "c",
    "node": 9
  }]
}'''
    
    print("Testing HUGR measurement handling...")
    
    # First, compile to QIR to see what it generates
    try:
        # Use the compiler directly
        compiler = RustHugrCompiler(llvm_convention='hugr')
        qir_str = compiler.compile_bytes_to_qir(hugr_bytes)
        print("\nGenerated QIR (HUGR convention):")
        print("=" * 60)
        print(qir_str)
        print("=" * 60)
        
        # Save it to a file for inspection
        with open('test_hugr_output.ll', 'w') as f:
            f.write(qir_str)
            
    except Exception as e:
        print(f"Failed to compile HUGR: {e}")
        return
    
    # Now try to run it
    try:
        print("\nCreating HUGR QIR Engine...")
        engine = RustHugrQirEngine(hugr_bytes, shots=10, llvm_convention='hugr')
        print(f"Engine created: {engine}")
        
        # Run with verbose output
        os.environ.pop('QIR_RUNTIME_QUIET', None)
        
        print("\nRunning quantum circuit...")
        results = engine.run()
        print(f"Got {len(results)} measurement results: {results}")
        
        # Check results
        unique_results = set(results)
        print(f"Unique results: {unique_results}")
        
        if len(unique_results) > 1:
            print("SUCCESS: Getting actual quantum measurements!")
        else:
            print("FAILURE: All measurements are the same value")
            
    except Exception as e:
        print(f"Failed to run HUGR: {e}")
        import traceback
        traceback.print_exc()

if __name__ == '__main__':
    test_simple_measurement()