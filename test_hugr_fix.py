#!/usr/bin/env python3
"""Test HUGR measurement fix"""

import os
os.environ['RUST_LOG'] = 'debug'
os.environ.pop('QIR_RUNTIME_QUIET', None)  # Enable runtime output

from pecos_rslib import RustHugrQirEngine

# Create a minimal valid HUGR with H gate and measurement
# This is based on the working Bell state example
hugr_json = {
    "version": "v1",
    "nodes": [
        {"parent": 0, "op": "Module"},
        {
            "parent": 0, 
            "op": "FuncDefn", 
            "name": "main",
            "signature": {
                "params": [],
                "body": {
                    "input": [],
                    "output": [{"t": "Sum", "s": "General", "rows": [[], []]}],
                    "extension_reqs": ["prelude", "tket2.quantum"]
                }
            }
        },
        {"parent": 1, "op": "Input", "types": []},
        {"parent": 1, "op": "Output", "types": [{"t": "Sum", "s": "General", "rows": [[], []]}]},
        {"parent": 1, "op": "DFG", "signature": {
            "input": [],
            "output": [{"t": "Sum", "s": "General", "rows": [[], []]}],
            "extension_reqs": ["prelude", "tket2.quantum"]
        }},
        {"parent": 4, "op": "Input", "types": []},
        {"parent": 4, "op": "Output", "types": [{"t": "Sum", "s": "General", "rows": [[], []]}]},
        {
            "parent": 4,
            "op": {"op_name": "QAlloc", "extension": "tket2.quantum", "args": [], "signature": {
                "input": [],
                "output": [{"t": "Q", "b": "B"}],
                "extension_reqs": []
            }}
        },
        {
            "parent": 4,
            "op": {"op_name": "H", "extension": "tket2.quantum", "args": [], "signature": {
                "input": [{"t": "Q", "b": "B"}],
                "output": [{"t": "Q", "b": "B"}],
                "extension_reqs": []
            }}
        },
        {
            "parent": 4,
            "op": {"op_name": "MeasureFree", "extension": "tket2.quantum", "args": [], "signature": {
                "input": [{"t": "Q", "b": "B"}],
                "output": [{"t": "Sum", "s": "General", "rows": [[], []]}],
                "extension_reqs": []
            }}
        }
    ],
    "edges": [
        [5, 0, 6, 0],
        [7, 0, 8, 0],
        [8, 0, 9, 0],
        [9, 0, 6, 0],
        [6, 0, 3, 0],
        [1, 0, 4],
        [4, 0, 1]
    ],
    "metadata": [{"name": "c", "node": 9}]
}

# Create HUGR bytes with proper magic number
import json
import struct

magic = struct.pack('>Q', 0x4855475269484A76)  # HUGR magic number (big-endian)
json_bytes = json.dumps(hugr_json).encode('utf-8')
hugr_bytes = magic + json_bytes

print(f"HUGR size: {len(hugr_bytes)} bytes")
print(f"First 16 bytes (hex): {hugr_bytes[:16].hex()}")

# Create engine and run
try:
    print("\nCreating HUGR QIR Engine...")
    engine = RustHugrQirEngine(hugr_bytes, shots=20, llvm_convention='hugr')
    print(f"Engine created: {engine}")
    
    print("\nRunning quantum circuit...")
    results = engine.run()
    print(f"Got {len(results)} measurement results")
    print(f"First 10 results: {results[:10]}")
    
    # Check results
    unique_results = set(results)
    print(f"\nUnique results: {unique_results}")
    
    if len(unique_results) > 1:
        print("✅ SUCCESS: Getting actual quantum measurements from PECOS infrastructure!")
        zeros = results.count(0)
        ones = results.count(1)
        print(f"Distribution: {zeros} zeros, {ones} ones")
        print(f"Ratio: {ones/len(results):.2f} (expected ~0.5 for H gate)")
    else:
        print("❌ FAILURE: All measurements are the same value")
        
except Exception as e:
    print(f"\nError: {e}")
    import traceback
    traceback.print_exc()