#!/usr/bin/env python3
"""Minimal Guppy runner - thin wrapper over Rust"""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.decorator import guppy as guppy_decorator

# Check what's available
import pecos_rslib
print("Available in pecos_rslib:", [x for x in dir(pecos_rslib) if 'hugr' in x.lower() or 'qir' in x.lower()])

# Import what we need
try:
    from pecos_rslib import compile_hugr_to_qir_rust
except ImportError:
    # Try alternative names
    from pecos_rslib import compile_hugr_to_llvm as compile_hugr_to_qir_rust

def run_guppy_minimal(guppy_func, shots=10, seed=None):
    """
    Minimal path: Guppy → HUGR → QIR → Execute
    No guards, no resets, no workarounds - just call Rust
    """
    func_name = getattr(guppy_func, 'name', 'guppy_function')
    print(f"=== Running {func_name} (minimal) ===")
    
    # Step 1: Compile Guppy to HUGR (Python side)
    print("1. Compiling Guppy → HUGR...")
    # Use guppy's compile_function
    compiled = guppy.compile_function(guppy_func)
    hugr_bytes = compiled.package.to_bytes()
    print(f"   ✅ Generated {len(hugr_bytes)} bytes of HUGR")
    
    # Step 2: Compile HUGR to QIR (Rust side)
    print("2. Compiling HUGR → QIR...")
    try:
        # Direct call to Rust - no intermediate layers
        qir_content = compile_hugr_to_qir_rust(
            hugr_bytes,
            None,  # output_path (use temp)
            False, # debug_info
            "hugr" # llvm_convention
        )
        print(f"   ✅ Generated QIR ({len(qir_content)} chars)")
    except Exception as e:
        print(f"   ❌ HUGR→QIR compilation failed: {e}")
        return None
    
    # Step 3: Execute QIR (Rust side)
    print("3. Executing QIR...")
    # For now, just save and use CLI since execute_qir has issues
    import tempfile
    with tempfile.NamedTemporaryFile(mode='w', suffix='.ll', delete=False) as f:
        f.write(qir_content)
        qir_file = f.name
    
    print(f"   Saved QIR to: {qir_file}")
    
    # Use CLI to execute (bypasses Python issues)
    import subprocess
    result = subprocess.run(
        ["cargo", "run", "-p", "pecos-cli", "--", "run", qir_file, 
         "--shots", str(shots), "--seed", str(seed or 42)],
        capture_output=True,
        text=True
    )
    
    if result.returncode == 0:
        print(f"   ✅ Execution successful!")
        print(f"   Results: {result.stdout.strip()}")
        return result.stdout.strip()
    else:
        print(f"   ❌ Execution failed: {result.stderr}")
        return None

# Import quantum operations at module level
from guppylang.std.quantum import qubit, h, cx, measure

# Test with a simple Bell state
@guppy_decorator
def bell_state() -> tuple[bool, bool]:
    q0 = qubit()
    q1 = qubit()
    h(q0)
    cx(q0, q1)
    return measure(q0), measure(q1)

if __name__ == "__main__":
    # Test the minimal path
    result = run_guppy_minimal(bell_state, shots=5, seed=42)
    
    if result:
        print("\n🎉 SUCCESS! Minimal path works!")
        print("This proves we don't need all the cruft!")
    else:
        print("\n❌ Failed - but we learned where the issue is")