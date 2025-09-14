#!/usr/bin/env python3
"""Simple test for Guppy → HUGR → QIR → PECOS pipeline.

Run with: uv run test_guppy_pipeline.py.
"""

import sys

sys.path.insert(0, "python/quantum-pecos/src")

from pecos.frontends import get_guppy_backends

# Check what's available
print(" Checking backends...")
backends = get_guppy_backends()
print(f" Guppy installed: {backends['guppy_available']}")
print(f" Rust backend: {backends['rust_backend']}")
# External tools are no longer tracked - only Rust backend is used

if backends["rust_backend"]:
    print("\n Infrastructure ready! Rust backend with HUGR support is available.")
else:
    print(f"\n  Rust backend issue: {backends.get('rust_message', 'Unknown')}")

# Try a simple classical function if Guppy is available
if backends["guppy_available"]:
    print("\n Testing Guppy compilation...")
    try:
        from guppylang.decorator import guppy

        @guppy
        def add(x: int, y: int) -> int:
            """Add two integers."""
            return x + y

        compiled = add.compile()
        print(" Classical function compiled successfully!")

    except (ImportError, AttributeError, RuntimeError) as e:
        print(f" Compilation failed: {e}")
        print("   (This is often due to guppylang API changes)")

print("\n To pin guppylang version, edit python/quantum-pecos/pyproject.toml:")
print('   Change: "guppylang>=0.19.0"')
print('   To:     "guppylang==0.19.1"')
