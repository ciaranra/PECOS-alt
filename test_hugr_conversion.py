#!/usr/bin/env python3
"""Test HUGR measurement conversion"""

from guppylang.decorator import guppy
from guppylang.std.quantum import h, measure, qubit

@guppy
def simple() -> bool:
    """Simple quantum circuit with measurement"""
    q = qubit()  
    h(q)
    return measure(q)

# Compile to HUGR
from guppylang import guppy as guppy_compiler
result = guppy_compiler.compile(simple)
print(f'Compiled result type: {type(result)}')

# Get HUGR bytes
# The result is a ModulePointer, we need to get the package bytes
pkg = result.package
hugr_bytes = pkg.to_bytes()
print(f'Generated HUGR: {len(hugr_bytes)} bytes')

# Now compile to QIR using Rust backend
from pecos_rslib import RustHugrCompiler
compiler = RustHugrCompiler(llvm_convention='hugr')

# This should trigger our debug output
try:
    qir = compiler.compile_bytes_to_qir(hugr_bytes)
    print('Compilation succeeded')
    print(f'QIR length: {len(qir)} chars')
    # Save it
    with open('guppy_generated.ll', 'w') as f:
        f.write(qir)
    # Check if it has measurement calls
    print(f'Contains measurement calls: {"__quantum__qis__m__body" in qir}')
    print(f'Contains call i32: {"call i32 @__quantum__qis__m__body(" in qir}')
    
    # Look for the actual calls
    for line in qir.split('\n'):
        if '__quantum__qis__m__body' in line and 'call' in line:
            print(f'Measurement line: {line.strip()}')
            
    # Now test running it
    print("\nTesting execution...")
    from pecos_rslib import RustHugrQirEngine
    
    engine = RustHugrQirEngine(hugr_bytes, shots=10, llvm_convention='hugr')
    results = engine.run()
    print(f"Results: {results}")
    print(f"Unique values: {set(results)}")
    
    if len(set(results)) > 1:
        print("✅ SUCCESS: Getting varied measurement results!")
    else:
        print("❌ FAILURE: All measurements are the same")
            
except Exception as e:
    print(f'Error: {e}')
    import traceback
    traceback.print_exc()